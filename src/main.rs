use anyhow::*;
use firmconfig::{
    cli::{Command, ProgramOptions},
    Attribute, AttributeType,
};
use std::{
    ffi::OsStr,
    io::{stdout, Write},
    path::{Path, PathBuf},
};
use structopt::StructOpt;

fn main() -> Result<()> {
    let options = ProgramOptions::from_args();
    match options.cmd {
        Command::Print {
            device_name,
            attribute,
        } => {
            do_for_device(device_name.as_deref(), |name| {
                print_device(&name, attribute.as_deref())
            })?;
        }
        Command::List { device_name } => {
            do_for_device(device_name.as_deref(), |name| list_device(name))?;
        }
        Command::Get {
            device_name,
            default,
            name,
            attribute,
        } => {
            do_for_device(device_name.as_deref(), |device_name| {
                get_attribute_value(device_name, &attribute, default, name)?
            })?;
        }
    }

    Ok(())
}

fn do_for_device<F, O>(device_name: Option<&OsStr>, f: F) -> Result<Option<O>>
where
    F: Fn(&OsStr) -> Result<O>,
{
    if let Some(name) = device_name {
        f(name).map(Some)
    } else {
        let path = Path::new("/sys/class/firmware-attributes");
        let mut o: Result<Option<O>> = Ok(None);
        for d in path.read_dir()? {
            if let Ok(d) = d {
                let r = f(&d.file_name()).map(Some);
                if let Ok(None) = o {
                    o = r
                } else if r.is_ok() {
                    o = r
                }
            }
        }
        o
    }
}

fn attributes_from(name: &OsStr) -> Result<Vec<Attribute>> {
    let mut path = PathBuf::from("/sys/class/firmware-attributes");
    path.push(name);

    firmconfig::list_attributes(&path)
}

fn get_attribute_value(
    device_name: &OsStr,
    attribute: &OsStr,
    default: bool,
    name: bool,
) -> Result<Result<(), Error>, Error> {
    if let Some(a) = get_attribute(device_name, attribute)? {
        if default {
            if let Ok(d) = a.default_value {
                println!("{}", d);
            } else {
                println!("<Access Denied>");
            }
        } else if name {
            println!("{}", a.display_name);
        } else if let Ok(d) = a.current_value {
            println!("{}", d);
        } else {
            println!("<Access Denied>");
        }
    }
    Ok(Ok(()))
}

fn get_attribute(device_name: &OsStr, name: &OsStr) -> Result<Option<Attribute>> {
    let attributes = attributes_from(device_name)?;

    for a in attributes {
        if a.name == name {
            return Ok(Some(a));
        }
    }

    Err(anyhow!("no attribute with name {}", name.to_string_lossy()))
}

fn list_device(name: &OsStr) -> Result<()> {
    let attributes = attributes_from(name)?;

    println!("Device: {}\n", name.to_string_lossy());
    for a in attributes {
        println!("{}: {}", a.name.to_string_lossy(), a.display_name);
    }

    Ok(())
}

fn print_device(name: &OsStr, attribute: Option<&OsStr>) -> Result<()> {
    let attributes = attributes_from(name)?;

    if let Some(attribute) = attribute {
        if let Some(a) = attributes.iter().find(|a| a.name == attribute) {
            println!("Device: {}\n", name.to_string_lossy());
            print_attribute(&a)?;
            return Ok(());
        } else {
            bail!("no attribute with name {}", attribute.to_string_lossy());
        }
    }

    println!("Device: {}\n", name.to_string_lossy());
    for a in attributes {
        print_attribute(&a)?;
    }
    Ok(())
}

fn print_attribute(a: &Attribute) -> Result<()> {
    let out = stdout();
    let mut f = out.lock();
    writeln!(f, "{}", a.name.to_string_lossy())?;
    writeln!(f, "    Name: {}", a.display_name)?;
    match a.tpe {
        AttributeType::Integer { min, max, step } => {
            writeln!(f, "    Type: Integer")?;
            writeln!(f, "        Min: {}", min)?;
            writeln!(f, "        Max: {}", max)?;
            writeln!(f, "        Step: {}", step)?;
        }
        AttributeType::String {
            min_length,
            max_length,
        } => {
            writeln!(f, "    Type: String")?;
            writeln!(f, "        Min: {}", min_length)?;
            writeln!(f, "        Max: {}", max_length)?;
        }
        AttributeType::Enumeration {
            ref possible_values,
        } => {
            writeln!(f, "    Type: Enumeration")?;
            writeln!(f, "        Possible Values:")?;
            for p in possible_values {
                writeln!(f, "            {}", p)?;
            }
        }
    }
    match &a.current_value {
        Ok(v) => {
            writeln!(f, "    Current value: {}", v)?;
        }
        Err(_) => {
            writeln!(f, "    Current value: <Access Denied>")?;
        }
    }
    match &a.default_value {
        Ok(v) => {
            writeln!(f, "    Default value: {}", v)?;
        }
        Err(_) => {
            writeln!(f, "    Default value: <Access Denied>")?;
        }
    }

    Ok(())
}
