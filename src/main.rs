use std::env;
use std::fs;
use std::time::{Duration, Instant};
use tokio::time::sleep;

// Archivo para guardar la última IP
const ARCHIVO_IP: &str = "ultima_ip.txt";

// Función para obtener la IP pública actual (async)
async fn obtener_ip_publica() -> Result<String, reqwest::Error> {
    let respuesta = reqwest::get("https://api.ipify.org").await?;
    if !respuesta.status().is_success()
    {
        let respuesta = reqwest::get("https://ipapi.co/ip").await?;

        return respuesta.text().await;
    }
    respuesta.text().await
}

// Función para enviar una notificación por Telegram
async fn enviar_notificacion(
    mensaje: &str,
    bot_token: &str,
    chat_id: i64,
) -> Result<(), reqwest::Error> {
    let params: [(&str, &str); 2] = [("chat_id", &chat_id.to_string()), ("text", mensaje)];
    let cliente = reqwest::Client::new();
    cliente
        .post(format!(
            "https://api.telegram.org/bot{}/sendMessage",
            bot_token
        ))
        .form(&params)
        .send()
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    // Leer la configuración de Telegram desde variables de entorno
    let bot_token =
        env::var("BOT_TOKEN").expect("La variable de entorno BOT_TOKEN no está definida");
    let chat_id: i64 = env::var("CHAT_ID")
        .expect("La variable de entorno CHAT_ID no está definida")
        .parse()
        .expect("CHAT_ID debe ser un número entero");
    let location: String = env::var("LOCATION").unwrap_or("LOCATION".to_string());
    // Leer la última IP y el tiempo del archivo si existe
    let mut ip_anterior = String::new();
    let mut tiempo_anterior = 0;
    if let Ok(contenido) = fs::read_to_string(ARCHIVO_IP) {
        let mut partes = contenido.split_whitespace();
        ip_anterior = partes.next().unwrap_or_default().to_string();
        tiempo_anterior = partes
            .next()
            .unwrap_or_default()
            .parse::<u64>()
            .unwrap_or(0);
    }

    loop {
        let ip_actual = obtener_ip_publica().await?;
        let tiempo_actual = Instant::now().elapsed().as_secs();

        if ip_actual != ip_anterior {
            let tiempo_transcurrido = (tiempo_actual - tiempo_anterior) / 3600; // Horas
            enviar_notificacion(
                &format!(
                    "Tu IP pública de {}, ha cambiado a: {} (después de {} horas)",
                    location, ip_actual, tiempo_transcurrido
                ),
                &bot_token,
                chat_id,
            )
            .await?;
            fs::write(ARCHIVO_IP, format!("{} {}", ip_actual, tiempo_actual))?;
            ip_anterior = ip_actual;
            tiempo_anterior = tiempo_actual;
        }

        sleep(Duration::from_secs(60)).await; // Verificar cada minuto
    }
}
