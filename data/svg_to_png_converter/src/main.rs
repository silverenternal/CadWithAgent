use image::{ImageBuffer, Rgba};
use resvg::tiny_skia;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn convert_svg_to_png(svg_path: &Path, png_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let svg_data = fs::read(svg_path)?;
    
    let options = usvg::Options::default();
    let fontdb = usvg::fontdb::Database::new();
    
    let tree = usvg::Tree::from_data(&svg_data, &options, &fontdb)?;
    
    let pixmap_size = tree.size();
    let width = pixmap_size.width() as u32;
    let height = pixmap_size.height() as u32;
    
    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or("Failed to create pixmap")?;
    
    resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(
        width,
        height,
        pixmap.data().to_vec(),
    ).ok_or("Failed to create image buffer")?;
    
    img.save(png_path)?;
    
    Ok(())
}

fn process_directory(
    input_dir: &Path,
    output_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut count = 0;
    let mut success = 0;
    let mut errors = 0;
    
    for entry in WalkDir::new(input_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "svg") {
            count += 1;
            
            // Get relative path from input_dir
            let relative_path = path.strip_prefix(input_dir)?;
            
            // Create output path with .png extension
            let mut output_path = output_dir.join(relative_path);
            output_path.set_extension("png");
            
            // Create parent directories if needed
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            println!("Converting: {:?} -> {:?}", path, output_path);
            
            match convert_svg_to_png(path, &output_path) {
                Ok(_) => {
                    success += 1;
                    println!("  ✓ Success");
                }
                Err(e) => {
                    errors += 1;
                    println!("  ✗ Error: {}", e);
                }
            }
        }
    }
    
    println!("\n=== Summary ===");
    println!("Total SVG files: {}", count);
    println!("Successful: {}", success);
    println!("Failed: {}", errors);
    
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    let (input_dir, output_dir) = if args.len() == 3 {
        (PathBuf::from(&args[1]), PathBuf::from(&args[2]))
    } else {
        // Default paths
        let base_dir = PathBuf::from("data/CubiCasa5k/data/cubicasa5k");
        (
            base_dir.join("svg"),
            base_dir.join("png"),
        )
    };
    
    println!("SVG to PNG Converter");
    println!("====================");
    println!("Input directory: {:?}", input_dir);
    println!("Output directory: {:?}", output_dir);
    
    if !input_dir.exists() {
        eprintln!("Error: Input directory does not exist: {:?}", input_dir);
        eprintln!("Please download the CubiCasa5K dataset from:");
        eprintln!("https://zenodo.org/record/2613548");
        eprintln!("And extract it to: data/CubiCasa5k/data/cubicasa5k/");
        std::process::exit(1);
    }
    
    fs::create_dir_all(&output_dir)?;
    
    process_directory(&input_dir, &output_dir)?;
    
    println!("\nConversion complete!");
    
    Ok(())
}
