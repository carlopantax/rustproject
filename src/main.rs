use std::error::Error;
use std::env;

mod caster;
mod receiver;
mod ui;

use receiver::receive_frame;
use ui::MyApp;  // Importa la struttura MyApp da ui.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Verifica gli argomenti della riga di comando per determinare se eseguire come caster, receiver o UI
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <caster|receiver|ui>", args[0]);
        std::process::exit(1);
    }

    // Se l'argomento è "caster", esegui il caster
    if args[1] == "caster" {
        let addr = "127.0.0.1:12345"; // Indirizzo su cui il caster ascolta
        println!("Avviando il caster...");
        caster::start_caster(addr).await?;
    }
    // Se l'argomento è "receiver", esegui il receiver
    else if args[1] == "receiver" {
        let addr = "127.0.0.1:12345"; // Indirizzo del caster
        let output_file = "received_frame.jpeg"; // File dove salvare il frame ricevuto
        println!("Avviando il receiver...");
        receive_frame(addr).await?;
    }
    // Se l'argomento è "ui", esegui l'interfaccia grafica
    else if args[1] == "ui" {
        let options = eframe::NativeOptions::default();
        eframe::run_native("Screencast App", options, Box::new(|_cc| Box::new(MyApp::default())))?;
    }
    // Se l'argomento non è valido, stampa un messaggio di errore
    else {
        eprintln!("Usage: {} <caster|receiver|ui>", args[0]);
        std::process::exit(1);
    }

    Ok(())
}

