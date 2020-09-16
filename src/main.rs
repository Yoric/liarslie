extern crate clap;
extern crate rand;
extern crate serde;
extern crate serde_derive;
extern crate tokio;

use rand::Rng;

mod agent;
mod conf;
mod play;
mod playexpert;
mod start;

#[tokio::main]
async fn main() {
    use clap::{Arg, SubCommand};
    let app = clap::App::new("Liars lie")
        .subcommand(
            SubCommand::with_name("start")
                .about("Start a number of agents, generate file agents.conf")
                .arg(
                    Arg::with_name("value")
                        .long("value")
                        .possible_value("true")
                        .possible_value("false"),
                )
                .arg(
                    Arg::with_name("num-agents")
                        .long("num-agents")
                        .value_name("number")
                        .default_value("10")
                        .validator(|s| {
                            s.parse::<usize>().map(|_| ()).map_err(|e| format!("{}", e))
                        }),
                )
                .arg(
                    Arg::with_name("liar-ratio")
                        .long("liar-ratio")
                        .value_name("ratio")
                        .default_value("0.1")
                        .validator(|s| match s.parse::<f64>() {
                            Err(e) => Err(format!("{}", e)),
                            Ok(v) if 0. <= v && v < 0.5 => Ok(()),
                            Ok(v) => Err(format!("Expected a value in [0., 0.5[, got {}", v)),
                        }),
                ),
        )
        .subcommand(
            SubCommand::with_name("play")
                .about("Play a single round of 'guess the original value'")
                .arg(
                    Arg::with_name("agents")
                        .long("agents")
                        .value_name("FILE")
                        .default_value("agents.conf"),
                ),
        )
        .subcommand(
            SubCommand::with_name("agent")
                .about("Start a single agent, print its port number on stdout")
                .arg(
                    Arg::with_name("value")
                        .long("value")
                        .takes_value(true)
                        .possible_value("true")
                        .possible_value("false")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("playexpert")
                .about("Play a single round of 'guess the original value', only talking to some agents")
                .arg(
                    Arg::with_name("agents")
                        .long("agents")
                        .value_name("FILE")
                        .default_value("agents.conf")
                )
                .arg(
                    Arg::with_name("liar-ratio")
                        .long("liar-ratio")
                        .value_name("ratio")
                        .default_value("0.1")
                        .validator(|s| match s.parse::<f64>() {
                            Err(e) => Err(format!("{}", e)),
                            Ok(v) if 0. <= v && v < 0.5 => Ok(()),
                            Ok(v) => Err(format!("Expected a value in [0., 0.5[, got {}", v)),
                        }),
                ),
        );

    match app.get_matches().subcommand() {
        ("start", Some(args)) => {
            let start_args = start::StartArgs {
                value: match args.value_of("value") {
                    None => rand::thread_rng().gen_bool(0.5),
                    Some(option) => option.parse::<bool>().expect("Invalud value: value"),
                },
                num_agents: args
                    .value_of("num-agents")
                    .expect("Missing arg: value")
                    .parse::<usize>()
                    .expect("Invalud value: value"),
                liar_ratio: args
                    .value_of("liar-ratio")
                    .expect("Missing arg: value")
                    .parse::<f64>()
                    .expect("Invalud value: value"),
            };
            assert!(start_args.liar_ratio >= 0.);
            assert!(start_args.liar_ratio < 0.5);
            start::start(&start_args).await;
        }
        ("agent", Some(args)) => {
            let agent_args = agent::AgentArgs {
                value: match args.value_of("value").expect("Missing arg: value") {
                    "true" => true,
                    "false" => false,
                    v => panic!("Invalid boolean {}", v),
                },
            };
            agent::agent(&agent_args).await;
            unreachable!();
        }
        ("play", Some(args)) => {
            let play_args = play::PlayArgs {
                path: args
                    .value_of("agents")
                    .expect("Missing arg: agents")
                    .parse::<std::path::PathBuf>()
                    .expect("Invalud value: agents"),
            };
            play::play(&play_args).await;
        }
        ("playexpert", Some(args)) => {
            let play_args = playexpert::PlayExpertArgs {
                path: args
                    .value_of("agents")
                    .expect("Missing arg: agents")
                    .parse::<std::path::PathBuf>()
                    .expect("Invalud value: agents"),
                liar_ratio: args
                    .value_of("liar-ratio")
                    .expect("Missing arg: value")
                    .parse::<f64>()
                    .expect("Invalud value: value"),
            };
            playexpert::play(&play_args).await;
        }
        _ => {
            panic!();
        }
    }
}
