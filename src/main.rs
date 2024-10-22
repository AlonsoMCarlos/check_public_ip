use std::env;
use std::fs;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::time::sleep;
// Archivo para guardar la última IP
const ARCHIVO_IP: &str = "/tmp/ultima_ip.txt";
const TIEMPO_NO_CAMBIO_HORA: u64 =  60; // 1 hora

use std::net::{AddrParseError, IpAddr};

fn parse_ip(ip: &str) -> Result<IpAddr, String> {
    let ip: IpAddr = ip
        .parse()
        .map_err(|error: AddrParseError| error.to_string())?;
    Ok(ip)
}

// Función para obtener la IP pública actual (async)
async fn get_public_ip_from(url: &str) -> Result<IpAddr, String> {
    let respuesta = reqwest::get(url).await.map_err(|error| error.to_string())?;
    if !respuesta.status().is_success() {
        return Err(respuesta.text().await.map_err(|error| error.to_string())?);
    }
    respuesta
        .text()
        .await
        .map_err(|error| error.to_string())
        .map(|ip| parse_ip(&ip))?
}

async fn get_public_ip() -> Result<String, String> {
    match get_public_ip_from("https://api.ipify.org").await {
        Ok(ip) => Ok(ip.to_string()),
        Err(_) => get_public_ip_from("https://ipapi.co/ip")
            .await
            .map(|ip| ip.to_string()),
    }
}

// Función para enviar una notificación por Telegram
async fn send_notification_to_telegram(
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
    let mut tiempo_no_cambio = TIEMPO_NO_CAMBIO_HORA;
    let mut time_to_plus = 1;
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
        let ip_actual = match get_public_ip().await {
            Ok(ip) => ip,
            Err(_) => {
                println!(
                    "[{}] Error al obtener la IP pública posible perdida de conexión a internet",
                    Utc::now().to_rfc3339()
                );
                sleep(Duration::from_secs(60)).await;
                continue;
            }
        };

        let tiempo_actual = Instant::now().elapsed().as_secs();
        if ip_actual == ip_anterior && tiempo_no_cambio == 0 {
            tiempo_no_cambio = time_to_plus * TIEMPO_NO_CAMBIO_HORA;
            time_to_plus += 1;
            let tiempo_transcurrido = (tiempo_actual - tiempo_anterior) / 3600; // Horas
            send_notification_to_telegram(
                &format!(
                    "Tu IP pública de {}, no ha cambiado en {} horas",
                    location, tiempo_transcurrido
                ),
                &bot_token,
                chat_id,
            )
            .await?;
        } else {
            tiempo_no_cambio -= 1;
        }
        if ip_actual != ip_anterior {
            let tiempo_transcurrido = (tiempo_actual - tiempo_anterior) / 3600; // Horas
            send_notification_to_telegram(
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
            tiempo_no_cambio = TIEMPO_NO_CAMBIO_HORA;
            time_to_plus = 1;
        }

        sleep(Duration::from_secs(60)).await; // Verificar cada minuto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_obtener_ip_publica() {
        let ip = get_public_ip().await.unwrap();

        assert!(!ip.is_empty(), "La IP no puede estar vacía");
    }
}
