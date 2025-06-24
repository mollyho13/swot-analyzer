// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::BufWriter;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use csv::ReaderBuilder;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct CompanyData {
    business_name: String,
    data: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaResponse {
    response: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct QuestionGenerationRequest {
    csv_path: String,
    business_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SWOTRequest {
    csv_path: String,
    pdf_path: String,
    business_name: String,
}

// Check if Ollama is running and has the required model
#[tauri::command]
async fn check_ollama_status() -> Result<String, String> {
    // Check if Ollama is installed and running
    let output = Command::new("ollama")
        .args(&["list"])
        .output()
        .map_err(|e| format!("Ollama not found. Please install Ollama first: {}", e))?;

    if !output.status.success() {
        return Err("Ollama is not running. Please start Ollama service.".to_string());
    }

    let models = String::from_utf8_lossy(&output.stdout);
    if !models.contains("llama3.2:3b") {
        return Err("llama3.2:3b model not found. Please run 'ollama pull llama3.2:3b' first.".to_string());
    }

    Ok("Ollama is ready with llama3.2:3b model".to_string())
}

// Call Ollama API locally
async fn call_ollama(prompt: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(180)) // Changed from 120 to 180 seconds
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // Significantly shorten the prompt to avoid timeouts
    let request = OllamaRequest {
        model: "llama3.2:3b".to_string(),
        prompt, // Now uses full prompt instead of shortened_prompt
        stream: false,
    };

    println!("Sending request to Ollama...");
    
    let response = client
        .post("http://localhost:11434/api/generate")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Ollama API error: {}", response.status()));
    }

    println!("Got response from Ollama, parsing...");

    let ollama_response: OllamaResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;

    println!("Successfully parsed Ollama response");

    Ok(ollama_response.response)
}

// Read CSV and extract company data
fn read_csv_data(csv_path: &str, business_name: &str) -> Result<CompanyData, String> {
    let file_content = fs::read_to_string(csv_path)
        .map_err(|e| format!("Failed to read CSV file: {}", e))?;

    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file_content.as_bytes());

    let headers = reader.headers()
        .map_err(|e| format!("Failed to read CSV headers: {}", e))?
        .clone();

    for result in reader.records() {
        let record = result.map_err(|e| format!("Failed to read CSV record: {}", e))?;
        
        // Look for business name column
        if let Some(_name_field) = record.iter().find(|field| 
            field.to_lowercase().contains(&business_name.to_lowercase())
        ) {
            let mut data = HashMap::new();
            
            for (i, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    data.insert(header.to_string(), field.to_string());
                }
            }
            
            return Ok(CompanyData {
                business_name: business_name.to_string(),
                data,
            });
        }
    }

    Err(format!("Business '{}' not found in CSV", business_name))
}

// Extract text from PDF
fn extract_pdf_text(pdf_path: &str) -> Result<String, String> {
    let bytes = fs::read(pdf_path)
        .map_err(|e| format!("Failed to read PDF file: {}", e))?;
    
    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| format!("Failed to extract text from PDF: {}", e))?;
    
    Ok(text)
}

#[tauri::command]
async fn generate_followup_questions(request: QuestionGenerationRequest) -> Result<Vec<String>, String> {
    // Read company data from CSV
    let company_data = read_csv_data(&request.csv_path, &request.business_name)?;
    
    // Build the company description
    let company_description = company_data.data
        .iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(key, value)| format!("{}: {}", key, value))
        .collect::<Vec<_>>()
        .join("\n");

    // Create a much shorter, more direct prompt
    let prompt = format!(r#"
Vous êtes analyste stratégique en France, spécialisé dans le diagnostic des PME. Votre mission consiste à générer un questionnaire personnalisé de 50 à 100 questions diagnostiques pour un dirigeant d'entreprise, à partir de ses réponses à un questionnaire de profilage général.

Given the following company details:

{}


L'objectif est de préparer une analyse SWOT (Forces, Faiblesses, Opportunités, Menaces) complète et structurée. Vos questions doivent explorer les axes stratégiques clés de l'entreprise avec précision et pertinence, en fonction de sa taille, de son secteur d'activité, de son chiffre d'affaires, de son modèle opérationnel, de sa structure clientèle et des défis déclarés.

Voici la marche à suivre :
1. Lire attentivement les 20 réponses du questionnaire de profilage. 2. Identifiez les caractéristiques clés de l'entreprise : modèle économique, stade de croissance, dynamique sectorielle, maturité numérique, exposition internationale, etc.
3. Sur cette base, élaborez 50 à 100 questions **personnalisées** sur les axes suivants et les questions devraient augmenter en complexité:
    - Stratégie commerciale (par exemple, performance commerciale, taux de désabonnement, pouvoir de fixation des prix)
    - Opérations et chaîne d'approvisionnement
    - Structure financière et marges
    - Positionnement sur le marché et concurrence
    - Ressources humaines et management
    - Outils numériques et transformation
    - Risques réglementaires et externes
    - Vision stratégique et projets d'avenir
4. Variez le type de questions (QCM, échelles de notation, texte court) mais n'incluez pas le type de question
5. Assurez-vous que chaque question contribue à révéler un élément concret pour l'analyse SWOT
6. ne regroupez pas les questions par axes

Soyez attentif au contexte : si l'entreprise externalise sa production, ne posez pas de questions sur les indicateurs clés de performance de la production interne ; s'il s'agit d'une activité B2B dans un secteur de niche, ne posez pas de questions sur l'image de marque grand public.
Ne posez pas de questions directes sur les forces, les faiblesses, les opportunités, les menaces ou autres choses de ce genre.

"#, company_description.chars().take(1000).collect::<String>());

    // Call Ollama
    let response = call_ollama(prompt).await?;
    
    // Parse questions from response
    let questions: Vec<String> = response
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Remove numbering and clean up
            let cleaned = line.trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ' ');
            cleaned.to_string()
        })
        .filter(|q| q.ends_with('?'))
        .take(90)
        .collect();

    Ok(questions)
}

#[tauri::command]
async fn generate_swot_analysis(request: SWOTRequest) -> Result<String, String> {
    // Read company data from CSV
    let company_data = read_csv_data(&request.csv_path, &request.business_name)?;
    
    // Extract PDF text
    let pdf_text = extract_pdf_text(&request.pdf_path)?;
    
    // Build company info
    let business_info = company_data.data
        .iter()
        .filter(|(_, value)| !value.trim().is_empty())
        .map(|(key, value)| format!("{}: {}", key, value))
        .collect::<Vec<_>>()
        .join("\n");

    // Create SWOT prompt (French version as in your original code)
    let prompt = format!(r#"
Réalise une analyse SWOT stratégique de l'entreprise {} en adoptant une approche consultante experte.

## CONSIGNES STRATÉGIQUES PRIORITAIRES

**Perspective d'analyse :** Adopte le point de vue d'un consultant senior qui comprend les enjeux spécifiques aux PME et les dynamiques sectorielles.

**Focus qualité > quantité :** Limite-toi aux 3-4 éléments les plus critiques par catégorie, mais développe-les avec profondeur stratégique.

## STRUCTURE D'ANALYSE

### ATOUTS (Forces)
Focus sur les **avantages concurrentiels réels** :
- Positionnement différenciant vs concurrents majeurs
- Modèle économique ou approche unique 
- Relations client et satisfaction (taille humaine, proximité)
- Expertise technique ou savoir-faire spécialisé
- Stabilité contractuelle ou récurrence business

### FAIBLESSES (Faiblesses internes)
**Identifier les risques opérationnels critiques** :
- Dépendances organisationnelles (leadership, personne-clé)
- Contraintes de structuration interne (processus, communication)
- Limitations financières impactant la croissance
- Vulnérabilités contractuelles ou commerciales majeures

### OPPORTUNITÉS
**Axes de développement réalistes** :
- Évolutions réglementaires/marché favorables au positionnement
- Opportunités de conquête commerciale identifiées
- Leviers de transformation digitale/innovation
- Possibilités d'expansion géographique ou diversification

### MENACES
**Risques business majeurs** :
- Concurrence spécifique (nommer les acteurs dominants)
- Complexité croissante du marché (appels d'offres, etc.)
- Risques financiers et de stabilité
- Évolutions défavorables de l'environnement d'affaires

## CRITÈRES DE QUALITÉ REQUIS

✅ **Spécificité sectorielle** : Montre une compréhension fine du secteur d'activité
✅ **Nommage précis** : Cite des concurrents, réglementations, ou modèles spécifiques quand pertinent  
✅ **Vision PME** : Intègre les enjeux typiques des PME (cash-flow, gouvernance, croissance vs. moyens)
✅ **Liens stratégiques** : Explique les implications business de chaque point
✅ **Priorisation** : Classe implicitement par ordre d'importance stratégique
✅ **Actionnable** : Chaque point doit suggérer des axes de travail concrets

## TONE ET STYLE

- **Langage consultant** : Précis, technique, orienté décision
- **Équilibre** : Ni trop optimiste ni pessimiste, réaliste
- **Synthèse** : Chaque point en 2-3 phrases maximum mais denses en insight


MISSION: Créer une analyse SWOT complète et structurée pour l'entreprise {}.

Réponses détaillées: {}

FORMAT DE RÉPONSE OBLIGATOIRE:


### FORCES (Atouts)
1. [Force 1 - description en 2-3 phrases]
2. [Force 2 - description en 2-3 phrases]  
3. [Force 3 - description en 2-3 phrases]
4. [Force 4 - description en 2-3 phrases]

### FAIBLESSES (Points d'amélioration)
1. [Faiblesse 1 - description en 2-3 phrases]
2. [Faiblesse 2 - description en 2-3 phrases]
3. [Faiblesse 3 - description en 2-3 phrases]
4. [Faiblesse 4 - description en 2-3 phrases]

### OPPORTUNITÉS (Possibilités de développement)
1. [Opportunité 1 - description en 2-3 phrases]
2. [Opportunité 2 - description en 2-3 phrases]
3. [Opportunité 3 - description en 2-3 phrases]
4. [Opportunité 4 - description en 2-3 phrases]

### MENACES (Risques externes)
1. [Menace 1 - description en 2-3 phrases]
2. [Menace 2 - description en 2-3 phrases]
3. [Menace 3 - description en 2-3 phrases]
4. [Menace 4 - description en 2-3 phrases]

INSTRUCTIONS:
- Analyser comme un consultant senior
- 4 points par catégorie exactement
- Être spécifique au secteur d'activité
- Utiliser un langage professionnel et technique
- Chaque point doit être actionnable
"#, request.business_name, business_info, pdf_text);

    // Call Ollama
    let swot_analysis = call_ollama(prompt).await?;
    
    Ok(swot_analysis)
}

#[tauri::command]
async fn save_questions_to_pdf(questions: Vec<String>, business_name: String, output_path: String) -> Result<String, String> {
    use printpdf::*;
    
    let (doc, page1, layer1) = PdfDocument::new(&format!("{} - Follow-up Questions", business_name), Mm(210.0), Mm(297.0), "Layer 1");
    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    
    // Add a font
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    
    // Better page layout settings
    let mut y_position = Mm(260.0); // Start lower to avoid header overlap
    let line_height = Mm(8.0); // More spacing between lines
    let page_width = Mm(240.0); // Narrower text width for better margins
    let left_margin = Mm(10.0); // Larger left margin
    let top_margin = Mm(260.0); // Consistent top margin
    let bottom_margin = Mm(40.0); // Larger bottom margin
    
    // Add title
    current_layer.use_text(format!("{} - Follow-up Questions", business_name), 16.0, left_margin, Mm(270.0), &font);
    y_position -= Mm(20.0); // Space after title
    
    // Helper function to wrap text with better character estimation
    fn wrap_text(text: &str, max_width: f32) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };
            
            // Better character width estimation for 12pt Helvetica: ~3.5 chars per mm
            if test_line.len() as f32 * 2.2 <= max_width {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
    
    for (i, question) in questions.iter().enumerate() {
        let question_text = format!("{}. {}", i + 1, question);
        let wrapped_lines = wrap_text(&question_text, page_width.0);
        
        for (line_idx, line) in wrapped_lines.iter().enumerate() {
            // Check if we need a new page
            if y_position < bottom_margin {
                let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                current_layer = doc.get_page(new_page).get_layer(new_layer);
                y_position = top_margin;
            }
            
            current_layer.use_text(line.clone(), 12.0, left_margin, y_position, &font);
            y_position -= line_height;
        }
        
        // Add extra space between questions
        y_position -= Mm(4.0);
    }
    
    let file = std::fs::File::create(&output_path).unwrap();
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).unwrap();
    
    Ok(format!("Questions saved to: {}", output_path))
}

#[tauri::command]
async fn save_swot_to_pdf(swot_text: String, business_name: String, output_path: String) -> Result<String, String> {
    use printpdf::*;
    
    let (doc, page1, layer1) = PdfDocument::new(&format!("{} - SWOT Analysis", business_name), Mm(210.0), Mm(297.0), "Layer 1");
    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    
    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    
    // Better page layout settings
    let mut y_position = Mm(260.0); // Start lower to avoid header overlap
    let line_height = Mm(7.0); // More spacing between lines
    let page_width = Mm(240.0); // Narrower text width for better margins
    let left_margin = Mm(10.0); // Larger left margin
    let top_margin = Mm(260.0); // Consistent top margin
    let bottom_margin = Mm(40.0); // Larger bottom margin
    
    // Add title
    current_layer.use_text(format!("{} - SWOT Analysis", business_name), 16.0, left_margin, Mm(270.0), &font);
    y_position -= Mm(20.0); // Space after title
    
    // Helper function to wrap text with better character estimation
    fn wrap_text(text: &str, max_width: f32) -> Vec<String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();
        
        for word in words {
            let test_line = if current_line.is_empty() {
                word.to_string()
            } else {
                format!("{} {}", current_line, word)
            };
            
            // Better character width estimation for 10pt Helvetica: ~4.2 chars per mm
            if test_line.len() as f32 * 2.2 <= max_width {
                current_line = test_line;
            } else {
                if !current_line.is_empty() {
                    lines.push(current_line);
                }
                current_line = word.to_string();
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        lines
    }
    
    // Split text into paragraphs first
    let paragraphs: Vec<&str> = swot_text.split('\n').collect();
    
    for paragraph in paragraphs {
        if paragraph.trim().is_empty() {
            // Empty line - add some space
            y_position -= line_height;
            continue;
        }
        
        let wrapped_lines = wrap_text(paragraph.trim(), page_width.0);
        
        for line in wrapped_lines {
            // Check if we need a new page
            if y_position < bottom_margin {
                let (new_page, new_layer) = doc.add_page(Mm(210.0), Mm(297.0), "Layer 1");
                current_layer = doc.get_page(new_page).get_layer(new_layer);
                y_position = top_margin;
            }
            
            current_layer.use_text(line, 10.0, left_margin, y_position, &font);
            y_position -= line_height;
        }
        
        // Add space between paragraphs
        y_position -= Mm(3.0);
    }
    
    let file = std::fs::File::create(&output_path).unwrap();
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer).unwrap();
    
    Ok(format!("SWOT analysis saved to: {}", output_path))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            check_ollama_status,
            generate_followup_questions,
            generate_swot_analysis,
            save_questions_to_pdf,
            save_swot_to_pdf
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}