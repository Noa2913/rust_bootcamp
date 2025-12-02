use clap::{Parser, CommandFactory};
use rand::Rng;
use std::collections::{BinaryHeap, HashMap};
use std::fs;
use std::cmp::Ordering;

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::ffi::OsStr;

type Coord = (usize, usize);

#[derive(Copy, Clone, Eq, PartialEq)]
struct State {
    cost: u32,
    position: Coord,
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost)
            .then_with(|| self.position.cmp(&other.position))
    }
}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn parse_map(map_data: &str) -> Option<Vec<Vec<u8>>> {
    let mut grid = Vec::new();
    let mut cols = 0;
    
    for line in map_data.lines() {
        let hex_values: Vec<&str> = line.split_whitespace().collect();
        if hex_values.is_empty() { continue; }

        let row: Vec<u8> = hex_values.iter()
            .filter_map(|s| u8::from_str_radix(s, 16).ok())
            .collect();

        if row.len() > 0 {
            if cols == 0 { cols = row.len(); }
            if row.len() != cols { return None; }
            grid.push(row);
        }
    }
    Some(grid)
}

fn generate_map(w: usize, h: usize) -> Vec<Vec<String>> {
    let mut rng = rand::thread_rng();
    let mut grid = Vec::with_capacity(h);

    for r in 0..h {
        let mut row = Vec::with_capacity(w);
        for c in 0..w {
            let value = match (r, c) {
                (0, 0) => 0u8,
                _ if r == h - 1 && c == w - 1 => 0xFFu8,
                _ => rng.gen_range(0x01..0xFE),
            };
            row.push(format!("{:02X}", value));
        }
        grid.push(row);
    }
    grid
}

fn dijkstra(grid: &Vec<Vec<u8>>, start: Coord, end: Coord) -> Option<(u32, Vec<Coord>)> {
    let rows = grid.len();
    let cols = grid[0].len();
    let mut dist: HashMap<Coord, u32> = HashMap::new();
    let mut predecessors: HashMap<Coord, Coord> = HashMap::new();
    let mut heap = BinaryHeap::new();

    dist.insert(start, 0);
    heap.push(State { cost: 0, position: start });

    let moves = [(0, 1), (0, -1), (1, 0), (-1, 0)];

    while let Some(State { cost, position }) = heap.pop() {
        if position == end {
            let total_cost = dist[&end];
            let mut path = Vec::new();
            let mut curr = end;
            
            while curr != start {
                path.push(curr);
                curr = predecessors[&curr];
            }
            path.push(start);
            path.reverse();
            return Some((total_cost, path));
        }

        if cost > dist[&position] {
            continue;
        }

        for (dr, dc) in moves.iter() {
            let new_r = (position.0 as isize) + dr;
            let new_c = (position.1 as isize) + dc;

            if new_r >= 0 && new_r < rows as isize && new_c >= 0 && new_c < cols as isize {
                let neighbor_pos = (new_r as usize, new_c as usize);
                let step_cost = grid[neighbor_pos.0][neighbor_pos.1] as u32;
                let new_total_cost = cost + step_cost;

                let current_dist = dist.get(&neighbor_pos).copied().unwrap_or(u32::MAX);
                
                if new_total_cost < current_dist {
                    dist.insert(neighbor_pos, new_total_cost);
                    predecessors.insert(neighbor_pos, position);
                    heap.push(State { cost: new_total_cost, position: neighbor_pos });
                }
            }
        }
    }

    None
}

fn max_path_dfs(grid: &Vec<Vec<u8>>, start: Coord, end: Coord) -> Option<(u32, Vec<Coord>)> {
    let rows = grid.len();
    let cols = grid[0].len();
    
    // Pour les grilles > 6x6, utiliser une heuristique glouton au lieu du DFS exhaustif
    if rows > 6 || cols > 6 {
        return greedy_max_path(grid, start, end);
    }

    let mut visited = vec![vec![false; cols]; rows];
    let mut best_cost: Option<u32> = None;
    let mut best_path: Vec<Coord> = Vec::new();
    let max_depth = ((rows * cols) / 2) as u32; // Limite réduite

    fn dfs(grid: &Vec<Vec<u8>>, pos: Coord, end: Coord,
           visited: &mut Vec<Vec<bool>>, path: &mut Vec<Coord>,
           cur_cost: u32, best_cost: &mut Option<u32>, best_path: &mut Vec<Coord>,
           rows: usize, cols: usize, depth: u32, max_depth: u32) {
        
        if depth > max_depth {
            return;
        }

        if pos == end {
            if best_cost.is_none() || cur_cost > best_cost.unwrap() {
                *best_cost = Some(cur_cost);
                *best_path = path.clone();
            }
            return;
        }

        let moves = [(0isize, 1isize), (0, -1), (1, 0), (-1, 0)];

        for (dr, dc) in moves.iter() {
            let nr = pos.0 as isize + dr;
            let nc = pos.1 as isize + dc;
            if nr >= 0 && nr < rows as isize && nc >= 0 && nc < cols as isize {
                let nr = nr as usize;
                let nc = nc as usize;
                if !visited[nr][nc] {
                    visited[nr][nc] = true;
                    path.push((nr, nc));
                    let step_cost = grid[nr][nc] as u32;
                    dfs(grid, (nr, nc), end, visited, path, cur_cost + step_cost, best_cost, best_path, rows, cols, depth + 1, max_depth);
                    path.pop();
                    visited[nr][nc] = false;
                }
            }
        }
    }

    visited[start.0][start.1] = true;
    let mut path = vec![start];
    dfs(grid, start, end, &mut visited, &mut path, 0u32, &mut best_cost, &mut best_path, rows, cols, 0, max_depth);

    best_cost.map(|c| (c, best_path))
}

// Heuristique glouton pour trouver un chemin de coût élevé (pas exhaustif)
fn greedy_max_path(grid: &Vec<Vec<u8>>, start: Coord, end: Coord) -> Option<(u32, Vec<Coord>)> {
    let rows = grid.len();
    let cols = grid[0].len();
    let moves = [(0, 1), (0, -1), (1, 0), (-1, 0)];
    
    let mut visited = vec![vec![false; cols]; rows];
    let mut path = vec![start];
    let mut cost = 0u32;
    let mut current = start;
    
    visited[start.0][start.1] = true;

    // Explore greedily vers les cellules de plus haute valeur
    while current != end {
        let mut best_next = None;
        let mut best_value = 0u8;
        
        for (dr, dc) in moves.iter() {
            let nr = (current.0 as isize + dr) as usize;
            let nc = (current.1 as isize + dc) as usize;
            
            if nr < rows && nc < cols && !visited[nr][nc] {
                let cell_value = grid[nr][nc];
                if cell_value > best_value {
                    best_value = cell_value;
                    best_next = Some((nr, nc));
                }
            }
        }
        
        match best_next {
            Some((nr, nc)) => {
                visited[nr][nc] = true;
                cost += grid[nr][nc] as u32;
                path.push((nr, nc));
                current = (nr, nc);
            }
            None => {
                // Si bloqué, chercher n'importe quel chemin non visité
                let mut found = false;
                for (dr, dc) in moves.iter() {
                    let nr = (current.0 as isize + dr) as usize;
                    let nc = (current.1 as isize + dc) as usize;
                    
                    if nr < rows && nc < cols && !visited[nr][nc] {
                        visited[nr][nc] = true;
                        cost += grid[nr][nc] as u32;
                        path.push((nr, nc));
                        current = (nr, nc);
                        found = true;
                        break;
                    }
                }
                if !found { break; }
            }
        }
    }

    if current == end {
        Some((cost, path))
    } else {
        None
    }
}

fn hex_to_rainbow_ansi(value: u8) -> String {
    let color_index = 16 + (value as f32 / 255.0 * 215.0).round() as u8;
    format!("\x1b[38;5;{}m", color_index)
}

fn visualize_map(grid_str: &Vec<Vec<String>>, path: Option<&Vec<Coord>>, path_color: &str) {
    let path_set = path.map(|p| p.iter().collect::<std::collections::HashSet<_>>()).unwrap_or_default();
    
    for (r, row) in grid_str.iter().enumerate() {
        for (c, hex_val) in row.iter().enumerate() {
            let value = u8::from_str_radix(hex_val, 16).unwrap_or(0);
            let color = hex_to_rainbow_ansi(value);
            
            let text_color = if !path_set.is_empty() && path_set.contains(&(r, c)) {
                path_color
            } else {
                "\x1b[0m"
            };

            print!("{}{}{} ", color, text_color, hex_val);
        }
        println!("\x1b[0m");
    }
}

fn print_path_details(name: &str, cost: u32, path: &Vec<Coord>, grid_u8: &Vec<Vec<u8>>) {
    println!("\n{} COST PATH (shown in {}):", name, if name == "MINIMUM" { "white" } else { "red" });
    println!("==========================");
    println!("Total cost: 0x{:X} ({} decimal)", cost, cost);
    println!("Path length: {} steps", path.len());
    
    print!("Path:\n");
    for (i, &(r, c)) in path.iter().enumerate() {
        print!("({},{})", r, c);
        if i < path.len() - 1 {
            print!("->");
        }
        if (i + 1) % 6 == 0 {
            print!("\n");
        }
    }
    println!();
    
    println!("\nStep-by-step costs:");
    println!("Start 0x{:02X} ({},{})", grid_u8[path[0].0][path[0].1], path[0].0, path[0].1);
    let mut _current_cost = 0;
    for i in 1..path.len() {
        let curr = path[i];
        let step_cost = grid_u8[curr.0][curr.1] as u32;
        _current_cost += step_cost;
        println!("-> 0x{:02X} ({},{}) +{}", grid_u8[curr.0][curr.1], curr.0, curr.1, step_cost);
    }
    println!("Total: 0x{:X} ({})", cost, cost);
}

#[derive(Parser, Debug)]
#[clap(name = "hexpath", version = "1.0", about = "Find min/max cost paths in hexadecimal grid using Dijkstra")]
struct Cli {
    #[arg(long)]
    generate: Option<String>,

    map_file: Option<String>,

    #[arg(long, value_name = "FILE")]
    output: Option<String>,

    #[arg(long)]
    visualize: bool,

    #[arg(long)]
    both: bool,

    #[arg(long)]
    animate: bool,
}

#[cfg(windows)]
fn enable_ansi_support() {
    use winapi::um::winbase::STD_OUTPUT_HANDLE;
    use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
    use winapi::um::processenv::GetStdHandle;
    
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) != 0 {
            mode |= 0x0004; // ENABLE_VIRTUAL_TERMINAL_PROCESSING
            SetConsoleMode(handle, mode);
        }
    }
}

#[cfg(not(windows))]
fn enable_ansi_support() {
    // Pas nécessaire sur Linux/Mac
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_ansi_support(); // Activer ANSI avant tout affichage
    
    let args = Cli::parse();
    let mut map_data_str = String::new();
    let mut grid_str_vec: Option<Vec<Vec<String>>> = None;

    if let Some(ref dims) = args.generate {
        let parts: Vec<&str> = dims.split('x').collect();
        if parts.len() == 2 {
            let w = parts[0].parse::<usize>().unwrap_or(8);
            let h = parts[1].parse::<usize>().unwrap_or(8);
            grid_str_vec = Some(generate_map(w, h));
            println!("Generating {}x{} hexadecimal grid...", w, h);
        }
    } else if let Some(filename) = &args.map_file {
        map_data_str = fs::read_to_string(filename)?;
        println!("Analyzing hexadecimal grid...");
    } else {
        Cli::command().print_help()?;
        return Ok(());
    }

    let grid_str_to_process = grid_str_vec.as_ref().map(|g| 
        g.iter().map(|r| r.join(" ")).collect::<Vec<String>>().join("\n")
    ).unwrap_or(map_data_str.clone());

    let grid_u8 = parse_map(&grid_str_to_process).ok_or("Invalid map format")?;
    
    if grid_str_vec.is_none() {
        grid_str_vec = Some(grid_u8.iter().map(|r| 
            r.iter().map(|&v| format!("{:02X}", v)).collect()
        ).collect());
    }

    let rows = grid_u8.len();
    let cols = grid_u8[0].len();
    let start = (0, 0);
    let end = (rows - 1, cols - 1);
    
    println!("Grid size: {}x{}", rows, cols);
    println!("Start: ({},0) = 0x{:02X}", start.0, grid_u8[start.0][start.1]);
    println!("End: ({},{}) = 0x{:02X}", end.0, end.1, grid_u8[end.0][end.1]);
    
    if args.generate.is_some() {
        println!("\nGenerated Map:");
        for row in grid_str_vec.as_ref().unwrap() {
            println!("{}", row.join(" "));
        }
    }
    
    if let Some(ref filename) = args.output {
        if args.generate.is_some() && grid_str_vec.is_some() {
            let output_content = grid_str_vec.as_ref().unwrap().iter()
                .map(|r| r.join(" "))
                .collect::<Vec<String>>()
                .join("\n");
            fs::write(filename, output_content)?;
            println!("\nMap saved to: {}", filename);
        }
    }
    
    let min_path_result = dijkstra(&grid_u8, start, end);
    let max_path_result = max_path_dfs(&grid_u8, start, end);

    // Si pas de flags, afficher par défaut les résultats
    let should_visualize = args.visualize || args.both || args.animate || 
                          (args.map_file.is_some() && !args.generate.is_some());

    if should_visualize {
        println!("\nHEXADECIMAL GRID (rainbow gradient):");
        println!("==================================================");
        visualize_map(grid_str_vec.as_ref().unwrap(), None, "");

        if let Some((cost, path)) = &min_path_result {
            println!("\nMINIMUM COST PATH (shown in WHITE):");
            visualize_map(grid_str_vec.as_ref().unwrap(), Some(path), "\x1b[37m");
            print_path_details("MINIMUM", *cost, path, &grid_u8);
        }

        if args.both {
            if let Some((cost, path)) = &max_path_result {
                println!("\nMAXIMUM COST PATH (shown in RED):");
                visualize_map(grid_str_vec.as_ref().unwrap(), Some(path), "\x1b[31m");
                print_path_details("MAXIMUM", *cost, path, &grid_u8);
            }
        }
    } else if let Some((cost, _)) = &min_path_result {
        println!("\nMinimum cost path found: 0x{:X} ({})", cost, cost);
    }

    Ok(())
}