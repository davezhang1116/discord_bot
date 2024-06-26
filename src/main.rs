use xml::reader::get_data;
use lazy_static::lazy_static;

// define static constants
lazy_static! {
    static ref FILE: String = get_data().database.clone(); 
    static ref TOKEN: String =  get_data().token.clone();
}

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use rand::Rng;

use rusqlite::{Connection, Result};

use serenity::async_trait;
use serenity::framework::standard::macros::{command, group, help, hook};
use serenity::framework::standard::{
    help_commands, Args, BucketBuilder, CommandGroup, CommandResult, Configuration, DispatchError, HelpOptions, StandardFramework
};
use serenity::gateway::ShardManager;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::{GatewayIntents, Ready};
use serenity::model::id::UserId;
use serenity::prelude::*;
use serenity::all::MessageBuilder;
use serenity::builder::CreateMessage;
use serenity::builder::{
    CreateButton,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::client::{Context, EventHandler};
use serenity::futures::StreamExt;

#[allow(deprecated)]
use serenity::utils::parse_username;

use anychain_bitcoin::BitcoinAddress;
use anychain_bitcoin::DogecoinTestnet;
use anychain_core::address::Address;

#[path = "./wallet/lib.rs"]
mod lib;

#[path = "./op_return/send.rs"]
mod op_return;

pub mod xml;

struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

struct CommandCounter;

impl TypeMapKey for CommandCounter {
    type Value = HashMap<String, u64>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        // confirms that the bot is connected to a Discord server 
        println!("{} is connected!", ready.user.name);
    }
}

// Define command groups and classify them

#[group]
#[commands(deposit, balance, send, tip, faucet)]
#[description = "A group with commands providing service related to the dogecoin testnet."]

struct General;

#[group]
#[commands(coinflip,mines)]
#[description = "Gambling commands."]
struct Gambling;

#[group]
#[commands(op_return_send)]
#[description = "OP_RETURN"]
#[allow(non_camel_case_types)]
struct OP_RETURN;

#[help]
#[individual_command_tip = "Hello! This is a dogecoin tipping bot that enables you to send, store and bet your TESTNET dogecoins. \nYou can find detailed usages by using !help <command>"]
#[command_not_found_text = "Could not find: `{}`."]
#[strikethrough_commands_tip_in_dm("")]
#[strikethrough_commands_tip_in_guild("")]
#[max_levenshtein_distance(3)]
#[indention_prefix = "+"]
#[lacking_permissions = "Hide"]
#[lacking_role = "Nothing"]
#[wrong_channel = "Strike"]

async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    // generate the help message
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[hook]
async fn before(ctx: &Context, msg: &Message, command_name: &str) -> bool {
    // log commands. 
    // NOTE: this program doesn't have a dashboard 
    println!("Got command '{}' by user '{}'", command_name, msg.author.name);
    let mut data: tokio::sync::RwLockWriteGuard<'_, TypeMap> = ctx.data.write().await;
    let counter: &mut HashMap<String, u64> = data.get_mut::<CommandCounter>().expect("Expected CommandCounter in TypeMap.");
    let entry = counter.entry(command_name.to_string()).or_insert(0);
    *entry += 1;
    true
}

#[hook]
async fn after(_ctx: &Context, _msg: &Message, command_name: &str, command_result: CommandResult) {
    // confirms whether the command has been executed without error
    match command_result {
        Ok(()) => println!("Processed command '{command_name}'"),
        Err(why) => println!("Command '{command_name}' returned error {why:?}"),
    }
}

#[hook]
async fn unknown_command(ctx: &Context, msg: &Message, unknown_command_name: &str) {
    // reply when the bot doesn't recognize a command
    let _ = msg.reply(ctx, format!("Could not find command named '{unknown_command_name}', you may find all available commands under !help")).await;
}

#[hook]
async fn normal_message(_ctx: &Context, _msg: &Message) {
    // you can log users' messages here
}

#[hook]
async fn delay_action(ctx: &Context, msg: &Message) {
    // set delay for claiming faucet
    let _ = msg.reply(ctx, "You may only claim the faucet once a day").await;
}

#[hook]
async fn dispatch_error(ctx: &Context, msg: &Message, error: DispatchError, _command_name: &str) {
    // prevent the bot from being overloaded
    if let DispatchError::Ratelimited(info) = error {
        if info.is_first_try {
            let _ = msg
                .channel_id
                .say(&ctx.http, &format!("Try this again in {} seconds.", info.as_secs()))
                .await;
        }
    }
}

struct Buttons{
    index: String,
    clicked: bool,
    bomb: bool,
    label: String
}

#[tokio::main]
async fn main() {
    // the main function 
    // connect to Discord and initialize all the bots' functions
    let http: Http = Http::new(&*TOKEN);
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            if let Some(team) = info.team {
                owners.insert(team.owner_user_id);
            } else if let Some(owner) = &info.owner {
                owners.insert(owner.id);
            }
            match http.get_current_user().await {
                Ok(bot_id) => (owners, bot_id.id),
                Err(why) => panic!("Could not access the bot id: {:?}", why),
            }
        },
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let framework: StandardFramework = StandardFramework::new()
        .before(before)
        .after(after)
        .unrecognised_command(unknown_command)
        .normal_message(normal_message)
        .on_dispatch_error(dispatch_error)
        .bucket("faucet", BucketBuilder::default().delay(86400).delay_action(delay_action)).await
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&GAMBLING_GROUP)
        .group(&OP_RETURN_GROUP);


    framework.configure(
        Configuration::new().with_whitespace(true)
            .on_mention(Some(bot_id))
            .prefix("!")
            .delimiters(vec![", ", ","])
            .owners(owners),
    );
    let intents: GatewayIntents = GatewayIntents::all();
    let mut client: Client = Client::builder(&*TOKEN, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<CommandCounter>(HashMap::default())
        .await
        .expect("Err creating client");

    {
        let mut data: tokio::sync::RwLockWriteGuard<'_, TypeMap> = client.data.write().await;
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
    }

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}


#[command]
#[description(
    r#"This command allows you to deposit TESTNET dogecoin from you wallet. 

Note: the address will expire after 5 min and will only be valid once. 

If you send to the same address twice or too late, you will not be credited.

DO NOT DEPOSIT MAINNET DOGECOIN, YOU WILL LOSE YOUR FUND."#)]

async fn deposit(ctx: &Context, msg: &Message) -> CommandResult{
    let address: String = match lib::get_new_address().await{
        Ok(address)=>{address},
        Err(_) => String::from("error")
    };

    if address.clone() == String::from("error"){
        msg.channel_id.say(&ctx.http, "error generating address").await?;
        return Ok(());
    }
    msg.channel_id.say(&ctx.http, &address.clone()).await?;
    msg.channel_id.say(&ctx.http, "This address will expire in 5 minutes. It can only be used once.").await?;

    let mut status: bool = false;
    for _i in 0..300{
        std::thread::sleep(std::time::Duration::from_millis(1000));
        let amount: f64 = lib::get_received_amount(address.clone()).await?;
        if amount != 0.0{
            let conn: Connection = Connection::open(&*FILE).unwrap();
            
            let sats: f64 = get_balance(&msg.author.name).unwrap_or(0.0);
            if sats != 0.0{
                status = true;
                conn.execute(
                    &format!("Update balance set sats = {} where name = \"{}\"",
                    sats + amount,&msg.author.name),()
                )?;
                
            }else{
                conn.execute(
                    "INSERT INTO balance (id, name, sats) VALUES (?1, ?2, ?3)",
                    (rand::thread_rng().gen_range(0..10000000), &msg.author.name, amount),
                )?;
            }
            msg.channel_id.say(&ctx.http, &format!("Received {} from {}", amount, &address)).await?;
            break;
        }
    }
    match status{
        false => {msg.channel_id.say(&ctx.http, &format!("Address {} is expired, do not send anything to that address", &address)).await?;}
        true => {}
    }
    return Ok(());
}


#[command]
#[description(
    r#"This command allows you to send your balance to other addresses in the dogecoin testing network. 

Note that the amount must be have at most 8 decimal places and you must send at least 10 coins"#)]
#[usage("<testnet dogecoin address> <amount to send>")]
#[example("nZ6oQPaD4NyuhF2pyMCU2Ju3oeTWitz4Xs 102.0")]
async fn send(ctx: &Context, msg: &Message, args: Args) -> CommandResult{
    let input: Vec<&str> = args.rest().split(" ").collect::<Vec<&str>>();
    if input.len() != 2{
        msg.reply(ctx, format!("invalid input")).await?;
    }else{
        let account: &str = input[0];
        let amt: &str = input[1];

        if amt.to_string().trim().parse::<f64>().is_ok(){
            let amount = amt.to_string().trim().parse::<f64>().unwrap();
            let amount_owned = get_balance(&msg.author.name).unwrap_or(0.0);
            if BitcoinAddress::<DogecoinTestnet>::is_valid(account) {
                if amount >= 10.0 {
                    if amount <= amount_owned{

                        let tx_hash: String = match lib::send(account.to_string(), amount).await{
                            Ok(tx_hash)=>{tx_hash},
                            Err(_) => String::from("error")
                        };

                        if tx_hash.clone() == String::from("error"){
                            msg.channel_id.say(&ctx.http, "error sending coins").await?;
                            return Ok(());
                        }

                        if tx_hash.len() == 64{
                            msg.reply(ctx, format!("tx: {}\n [view transaction in explorer]({})", &tx_hash, format!("https://sochain.com/tx/DOGETEST/{}", &tx_hash))).await?;
                            let conn: Connection = Connection::open(&*FILE).unwrap();
                        
                            conn.execute(
                                &format!("Update balance set sats = {} where name = \"{}\"",
                                amount_owned - amount - 1.0, &msg.author.name),()
                            )?;
                        }else{
                            msg.reply(ctx, format!("error send coins, try again later")).await?;
                        }
                    }else{
                        msg.reply(ctx, format!("not enough balance to send the transaction and pay the transaction fee")).await?;
                    }
                }else{
                    msg.reply(ctx, format!("minimum sending amount is 10 coins")).await?;
                }                        
            } else {
                msg.reply(ctx, format!("invalid address")).await?;
            }
        }else{
            msg.reply(ctx, format!("invalid input")).await?;
        }
    }
    Ok(())
    
}


fn get_balance (username:&str) -> Result<f64,>{
    // get user balance from the database
    let conn: Connection = Connection::open(&*FILE).unwrap();

    struct Balance{sats: String}
    let mut stmt: rusqlite::Statement<'_> = conn.prepare(&format!("SELECT sats FROM balance where name = \"{}\"", &username))?;
    let person_iter = stmt.query_map([], |row| {
        Ok(Balance{
            sats: row.get(0)?
        })
    })?;

    let mut coin: f64 = 0.0;

    for i in person_iter{
        match i{
            Ok(sats) => {
                coin = sats.sats.parse::<f64>().unwrap();
            },
            Err(_) => {
                coin = 0.0;
            }
        };
    }
    Ok(coin)
}

#[command]
#[description(
    r#"This command allows you to send your balance to other addresses in the dogecoin TESTNET. 

Note that the amount must be have at most 8 decimal places and you must tip at least 10 coins."#)]
#[usage("<target user mention> <amount to tip>")]
#[example("@dave 120")]

async fn tip(ctx: &Context, msg: &Message, args: Args) -> CommandResult{

    // parse input

    let input = args.rest().split(" ").collect::<Vec<&str>>();
    if input.len() != 2{
        msg.reply(ctx, format!("invalid input")).await?;
    }else{
        let account: &str = input[0];
        let amt: &str = input[1];
        if amt.to_string().trim().parse::<f64>().is_ok(){
            let amt: f64 = amt.to_string().trim().parse::<f64>().unwrap();
            if amt >= 10.0{
                let balance = get_balance(&msg.author.name).unwrap_or(0.0);
                if  balance> amt.to_string().trim().parse::<f64>().unwrap(){
                    #[allow(deprecated)]
                    if !parse_username(account).is_none(){

                        // if all conditions are met, the bot will edit the balance
                        let conn: Connection = Connection::open(&*FILE).unwrap();
                        let username: &String = &parse_username(account).unwrap().to_user(&ctx.http).await.unwrap().name;
                        let sats: f64 = get_balance(username).unwrap_or(0.0);
                        if sats != 0.0{
                            conn.execute(
                                &format!("Update balance set sats = {} where name = \"{}\"",
                                sats + amt, username),()
                            )?;
                            
                        }else{
                            conn.execute(
                                "INSERT INTO balance (id, name, sats) VALUES (?1, ?2, ?3)",
                                (rand::thread_rng().gen_range(0..10000000), username, amt),
                            )?;
                        }

                        conn.execute(
                            &format!("Update balance set sats = {} where name = \"{}\"",
                               balance - amt, &msg.author.name),()
                        )?;
                        let response = MessageBuilder::new()
                            .push_bold_safe(&msg.author.name)
                            .push(" tipped ")
                            .push_bold_safe(username)
                            .push(format!(" {} coins", amt))
                            .build();

                        msg.reply(ctx, response).await?;
                    }else{
                        msg.reply(ctx, format!("invalid username")).await?;
                    }    
                }else{
                    msg.reply(ctx, format!("not enough balance")).await?;
                }
            }else{
                msg.reply(ctx, format!("minimum tipping amount is 10 coins")).await?;
            }
                
        }else{
            msg.reply(ctx, format!("input a corrent amount")).await?;
        }
    }
    Ok(())
}


#[command]
#[description(
    r#"This command gives you 500 coins for free. You can only claim it once a day."#)]
#[usage("")]
#[bucket = "faucet"]
async fn faucet(ctx: &Context, msg: &Message) -> CommandResult{
    
    // gave user free coins

    let username: &String = &msg.author.name;
    let amt: f64 = 500.0;
    let balance: f64 = get_balance(&msg.author.name).unwrap_or(0.0);
    let conn: Connection = Connection::open(&*FILE).unwrap();
    if balance != 0.0{
        conn.execute(
            &format!("Update balance set sats = {} where name = \"{}\"",
            balance + amt, username),()
        )?;
        
    }else{
        conn.execute(
            "INSERT INTO balance (id, name, sats) VALUES (?1, ?2, ?3)",
            (rand::thread_rng().gen_range(0..10000000), username, amt),
        )?;
    }
    msg.reply(ctx, format!("you have claimed 500 coins from the faucet, enjoy.")).await?;
    Ok(())
}


#[command]
#[description("This command prints the amount of testnet dogecoin that you own")]
async fn balance(ctx: &Context, msg: &Message) -> CommandResult{
    // return user balance
    let username: String = msg.author.name.clone();
    let coin: f64 = get_balance(&username).unwrap_or(0.0);
    msg.reply(ctx, format!("you have {} dogecoin", coin)).await?;
    Ok(())
}


#[command]
#[description(
    r#"This is a command that performs coinflip. Min bet is 10."#)]
#[usage("<up or down> <amount to bet>")]
#[example("up 20")]
pub async fn coinflip(ctx: &Context, msg: &Message, args: Args) -> CommandResult{
    // parse input
    let input: &Vec<&str> = &args.rest().split(" ").collect::<Vec<&str>>();
    if input.len() != 2 && (input[0] != "up" || input[0] != "down"){
        msg.reply(ctx, format!("invalid input")).await?;
    }else{
        let bet: &str = input[0];
        let amt: &str = input[1];
        if amt.to_string().trim().parse::<f64>().is_ok(){
            let amt = amt.to_string().trim().parse::<f64>().unwrap();
            if amt >= 10.0{
                let balance = get_balance(&msg.author.name).unwrap_or(0.0);
                if balance < amt{
                    msg.reply(ctx, format!("Please input a smaller bet. You have {} coins", balance)).await?;
                }else{
                    let coin = rand::thread_rng().gen_range(0..2);
                    let side = |coin| {
                        if coin == 1{
                            return "up";
                        }else{
                            return "down";
                        }
                    };
                    let conn = Connection::open(&*FILE).unwrap();
                    if bet == side(coin) {
                        msg.reply(ctx, format!("The result is {}, you won {} coins", side(coin), amt)).await?;
                        conn.execute(
                            &format!("Update balance set sats = {} where name = \"{}\"",
                            balance + amt, &msg.author.name),()
                        )?;
                    }else{
                        msg.reply(ctx, format!("The result is {}, you lost {} coins", side(coin), amt)).await?;
                        conn.execute(
                            &format!("Update balance set sats = {} where name = \"{}\"",
                            balance - amt, &msg.author.name),()
                        )?;
                    }
                }
            }else{
                msg.reply(ctx, format!("Minimum bet is 10")).await?;
            }
        }else{
            msg.reply(ctx, format!("input a valid bet amount")).await?;
        }
    }
    Ok(())
}


#[command]
#[description(
    r#"This is a command that performs mines. Min amount of coins to bet is 10.
    
Due to a limitation on discord, there are only 20 mines instead of the usual 25 mines."#)]
#[usage("<amount of mines> <amount to bet>")]
#[example("5 20")]
pub async fn mines(ctx: &Context, msg: &Message, args: Args) -> CommandResult{
    // parse input
    let input: &Vec<&str> = &args.rest().split(" ").collect::<Vec<&str>>();
    if input.len() != 2 {
        msg.reply(ctx, format!("invalid input")).await?;
        return Ok(());
    }
    let bet: &str = input[0];
    let amt: &str = input[1];
    
    // check user balance
    
    let balance: f64 = get_balance(&msg.author.name).unwrap_or(0.0);

    if amt.to_string().trim().parse::<f64>().is_ok(){
            let amt: f64 = amt.to_string().trim().parse::<f64>().unwrap();
            if amt >= 10.0{
                
                if balance < amt{
                    msg.reply(ctx, format!("Please input a smaller bet. You have {} coins", balance)).await?;
                    return Ok(());
                }
            }else{
                msg.reply(ctx, format!("Minimum bet is 10")).await?;
                return Ok(());
            }
        }else{
            msg.reply(ctx, format!("Invalid amount")).await?;
            return Ok(());
        }
    println!("{bet}");

    // check if input is valid
    if bet.to_string().trim().parse::<i8>().is_ok(){
        if bet.to_string().trim().parse::<i8>().unwrap() == 0 || bet.to_string().trim().parse::<i8>().unwrap() > 19{
            msg.reply(ctx, "enter a valid bomb number").await?;
            return Ok(());
        }
    }else{
        msg.reply(ctx, "enter a valid bet").await?;
        return Ok(());
    }

    // multiplier calculator function
    
    let calculate_multiplier = |mines:i8, slots:i8| -> f64{
        let mut max = 20;
        let mut multiplier = 1.0;
        loop{
            if max == slots{
                break;
            }
            multiplier = multiplier / ((max - mines) as f64 / max as f64);
            max -= 1;
        }
        multiplier
    };

    // start a vector with bomb structs

    let mine: i8 = bet.to_string().trim().parse::<i8>().unwrap();

    let mut bomb_list: Vec<i32> = vec![];
    loop{
        if bomb_list.len() as i8 == mine{
            break;
        }
        let num: i32 = rand::thread_rng().gen_range(0..20);
        if !bomb_list.contains(&num){
            bomb_list.push(num);
        }
    }

    let mut list = vec![];
    for i in 1..21{
        if bomb_list.contains(&(i-1)){
            list.push(
                Buttons{
                    index:i.to_string(),
                    clicked: false,
                    bomb:true,
                    label: (i).to_string()
                }
            );
        }else{
            list.push(
                Buttons{
                    index:i.to_string(),
                    clicked:false,
                    bomb:false,
                    label: (i).to_string()
                }
            );
        }
    }
    println!("{:?}",bomb_list);

    // create a reply message with the game

    let mut c: CreateMessage = CreateMessage::new();
    for i in 0..20{
        c = c.button(CreateButton::new(&list[i].index).label(&list[i].label))
    }
    let m = msg
            .channel_id
            .send_message(
                &ctx,
                c.content("your earning is 0").button(CreateButton::new("cash_out").label("cash out").disabled(true))

            )
            .await?;

    // timeout 1 Hour
    let mut interaction_stream = m.await_component_interaction(&ctx.shard).timeout(Duration::from_secs(60 * 60)).stream();
    
    let round_numbers = |number: f64| -> f64{
        (format!("{:.02}", number)).trim().parse::<f64>().unwrap()
    };

    #[allow(unused_variables)]
    let edit_balance = |usename: &str, amt: f64|{
        let conn = Connection::open(&*FILE).unwrap();
        conn.execute(
            &format!("Update balance set sats = {} where name = \"{}\"",
            balance + amt, &msg.author.name),()
        ).unwrap();
    };

    // substract bet amount from balance
    edit_balance(&msg.author.name, -amt.to_string().trim().parse::<f64>().unwrap());

    // main game
    while let Some(interaction) = interaction_stream.next().await {
        let input: std::prelude::v1::Result<usize, std::num::ParseIntError> = interaction.data.custom_id.trim().parse::<usize>();
        let mut slots = 0;
            for i in 0..20{
                if list[i].clicked == true{
                    slots += 1
                }
            }
        if input.is_err(){
            let mut c: CreateInteractionResponseMessage = CreateInteractionResponseMessage::default();
            for i in 0..20{
                if list[i].bomb == true{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(true).style(serenity::all::ButtonStyle::Danger));
                }else if list[i].clicked == true{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(true).style(serenity::all::ButtonStyle::Success));
                }else{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(true));
                }
                
            }
            // if the player decide to cashout, the game will stop and the player will receive their reward
            let amount_won = round_numbers(amt.to_string().trim().parse::<f64>().unwrap() * calculate_multiplier(mine, 20 - slots));
            interaction
                .create_response(
                    &ctx,
                    CreateInteractionResponse::UpdateMessage(
                        c.content(
                            &format!("your earning is {}", amount_won)
                        )
                    .button(CreateButton::new("cash_out")
                    .label("cash out").disabled(true)))
                )
                .await
                .unwrap();

            msg.reply(ctx, format!("you won {} coins", amount_won)).await?;
            edit_balance(&msg.author.name, amount_won);
            break;
        }
        let idx = input.unwrap()-1;

        if list[idx].bomb == true{ 
            // if player clicked a bomb, he loses the coins
            let mut c: CreateInteractionResponseMessage = CreateInteractionResponseMessage::default();
            for i in 0..20{
                if list[i].bomb == true{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(true).style(serenity::all::ButtonStyle::Danger));
                }else if list[i].clicked == true{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(true).style(serenity::all::ButtonStyle::Success));
                }else{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(true));
                }
                
            }
            interaction
                .create_response(
                    &ctx,
                    CreateInteractionResponse::UpdateMessage(
                        c.content(
                            &format!("your earning is 0")
                        )
                    .button(CreateButton::new("cash_out")
                    .label("cash out").disabled(true)))
                )
                .await
                .unwrap();
            msg.reply(ctx, format!("you hit a bomb! You lost {amt} coins")).await?;
            break;
        }else{
            // if player doesn't hit a bomb, the multiplier will be updated
            list[idx].clicked = true;
            let mut c: CreateInteractionResponseMessage = CreateInteractionResponseMessage::default();
            let mut slots = 0;
            for i in 0..20{
                if list[i].clicked == true{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label).disabled(list[i].clicked).style(serenity::all::ButtonStyle::Success));
                }else{
                    c = c.button(CreateButton::new(&list[i].index).label(&list[i].label));
                }
                if list[i].clicked == true{
                    slots += 1
                }
            }
            
            if slots == 20-mine{
                // when the player hit all of the spots, the game will automatically end and the user will receive rewards
                let amount_won = round_numbers(amt.to_string().trim().parse::<f64>().unwrap() * calculate_multiplier(mine, 20 - slots));
                msg.reply(ctx,format!("you won {} coins", amount_won)).await?;
                interaction
                .create_response(
                    &ctx,
                    CreateInteractionResponse::UpdateMessage(c.content(&format!("your earning is {}", amount_won))
                    .button(CreateButton::new("cash_out").label("cash out").disabled(true)))
                )
                .await
                .unwrap();
                edit_balance(&msg.author.name, amount_won);
                break;
            }

            interaction
                .create_response(
                    &ctx,
                    CreateInteractionResponse::UpdateMessage(
                        c.content(
                            &format!("your earning is {}, next multiplier: {}", 
                            round_numbers(amt.to_string().trim().parse::<f64>().unwrap() * calculate_multiplier(mine, 20 - slots)),
                            round_numbers(calculate_multiplier(mine, 20 - slots - 1))
                        )
                    )
                    .button(CreateButton::new("cash_out").label("cash out").disabled(false)))
                )
                .await
                .unwrap();
        }
    }
    Ok(())
}


#[command]
#[description(
    r#"Save your message on dogecoin network. You may check your messages on the block explorer or through network nodes. You may only use utf-8 characters."#)]
#[usage("<your message>")]
#[example("HELLO WORLD")]
pub async fn op_return_send(ctx: &Context, msg: &Message, args: Args) -> CommandResult{
    #[allow(unused_assignments)]
    let mut message = String::from("");

    let file = &msg.attachments.iter().next(); // get the txt file url

    if args.rest() == "" && file.is_some() && file.unwrap().content_type == Some(String::from("text/plain; charset=utf-8")){
            let body = reqwest::get(&file.unwrap().url)
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            message = format!("{}", body); // stream the txt file from discord
    }else{
        message = String::from(args.rest()); // if there is no file, then read the text
    }

    if message == ""{
        msg.reply(ctx, format!("no input")).await?; // reply no input if there is no input or file
        Ok(())
    }else if message.len() > 100 * 1024{
        msg.reply(ctx, format!("message is larger than 100 kb")).await?; 
        Ok(())
    }else{      
        match op_return::send(String::from(message), None, None, None){
            Ok(tx_hash) =>{ 
                if tx_hash.len() == 64{
                    msg.reply(ctx, format!("tx: {}\n [view transaction in explorer]({})", &tx_hash, format!("https://sochain.com/tx/DOGETEST/{}", &tx_hash))).await? // return the explorer link
                }else{
                    msg.reply(ctx, format!("The backend returned an error. The size of the message may be too big or the backend is not operational. Try again later!")).await? // error if node returns an error json
                }},
            Err(_e) => msg.reply(ctx, format!("Unknown Error. Try again later.")).await? // return error if the tx creation failed
        };
        Ok(())
    }
}

