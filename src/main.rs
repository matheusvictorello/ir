#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(private_interfaces)]

// 1: Oferecer direito de subscrição de uma ação ordinária (ON)
// 2: Direito de subscrição de uma ação preferencial (PN)
// 3: Ação ordinária
// 4: Ação preferencial
// 5, 6, 7 e 8: Ações preferenciais de classes diferentes. Os códigos finalizados em 5, 6, 7 e 8 são ações preferenciais, mas pertencem às classes distintas: A (PNA), B (PNB), C (PNC) e D (PND), respectivamente.
// 9: Recibo de subscrição sobre ações ordinárias
// 10: Recibo de subscrição de ações preferenciais
// 11: Unidades e BDR
// 34: BDR
// F: fracionário

use calamine::{
    open_workbook,
    DeError,
    Error,
    Xlsx,
    Range,
    Reader,
    RangeDeserializerBuilder,
};
use chrono::{
    NaiveDate,
};
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
};
use serde::de::{
    Error as SerdeError,
    Visitor,
};
use serde::de::value::StrDeserializer;
use std::collections::HashMap;
use std::fmt;
use std::fs;

const BASE_DIR:     &'static str = "../eu-tudo/eu-negociacao";
const DATE_FORMAT:  &'static str = "%d/%m/%Y";
const MOVES:        &'static str = "Movimentação";
const NEGOCIATIONS: &'static str = "Negociação";

fn date_format<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    NaiveDate::parse_from_str(&s, DATE_FORMAT)
        .map_err(serde::de::Error::custom)
}

fn de_opt_f64<'de, D>(de: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    use calamine::Data::Float;

    let data = calamine::Data::deserialize(de);

    match data {
        Ok(Float(f)) => Ok(Some(f)),
        _            => Ok(None),
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Deserialize, Debug)]
enum InOut {
    #[serde(rename(deserialize = "Compra"))]
    #[serde(rename(deserialize = "Credito"))]
    In,
    
    #[serde(rename(deserialize = "Debito"))]
    #[serde(rename(deserialize = "Venda"))]
    Out,
}

#[derive(PartialEq, Eq, Clone, Deserialize, Debug)]
enum MoveType {
    #[serde(rename(deserialize = "Atualização"))]
    Atualizacao,
    
    #[serde(rename(deserialize = "Bonificação em Ativos"))]
    BonificacaoEmAtivos,

    #[serde(rename(deserialize = "Cessão de Direitos"))]
    CessaoDeDireitos,
    
    #[serde(rename(deserialize = "Cessão de Direitos - Solicitada"))]
    CessaoDeDireitosSolicitada,
    
    #[serde(rename(deserialize = "COMPRA / VENDA"))]
    CompraVenda,

    #[serde(rename(deserialize = "Desdobro"))]
    Desdobro,

    #[serde(rename(deserialize = "Direito de Subscrição"))]
    DireitosDeSubscricao,
    
    #[serde(rename(deserialize = "Direitos de Subscrição - Excercído"))]
    DireitosDeSubscricaoExercido,

    #[serde(rename(deserialize = "Direitos de Subscrição - Não Exercido"))]
    DireitosDeSubscricaoNaoExercido,
    
    #[serde(rename(deserialize = "Direito Sobras de Subscrição - Não Exercido"))]
    DireitoSobrasDeSubscricaoNaoExercido,

    #[serde(rename(deserialize = "Dividendo"))]
    Dividendo,

    #[serde(rename(deserialize = "Empréstimo"))]
    Emprestimo,

    #[serde(rename(deserialize = "Fração em Ativos"))]
    FracaoEmAtivos,

    #[serde(rename(deserialize = "Grupamento"))]
    Grupamento,

    #[serde(rename(deserialize = "Incorporação"))]
    Incorporacao,

    #[serde(rename(deserialize = "Juros Sobre Capital Próprio"))]
    JurosSobreCapitalProprio,

    #[serde(rename(deserialize = "Leilão de Fração"))]
    LeilaoDeFracao,

    #[serde(rename(deserialize = "Recibo de Subscrição"))]
    ReciboDeSubscricao,
    
    #[serde(rename(deserialize = "Reembolso"))]
    Reembolso,

    #[serde(rename(deserialize = "Rendimento"))]
    Rendimento,
    
    #[serde(rename(deserialize = "Solicitação de Subscrição"))]
    SolicitacaoDeSubscricao,
    
    #[serde(rename(deserialize = "Transferência"))]
    Transferencia,
    
    #[serde(rename(deserialize = "Transferência - Liquidação"))]
    TransferenciaLiquidacao,

    #[serde(rename(deserialize = "VENCIMENTO"))]
    Vencimento,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Deserialize, Debug)]
struct Asset {
    code:   String,
    number: u32,
    f:      bool,
    t:      bool,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "", code, number, ft)
    }
}

#[derive(Clone, Deserialize, Debug)]
enum Broker {
    #[serde(rename(deserialize = "CLEAR CORRETORA - GRUPO XP"))]
    Clear,
    #[serde(rename(deserialize = "NU INVEST CORRETORA DE VALORES S.A."))]
    NuInvest,
    #[serde(rename(deserialize = "XP INVESTIMENTOS CCTVM S/A"))]
    XP,
}

pub fn asset_format<'de, D>(de: D) -> Result<Asset, D::Error>
where
    D: Deserializer<'de>,
{
    let     s = String::deserialize(de)?;
    let mut s = s.split(" - ");
    let     s = s.next();

    let Some(s) = s else {
        return Err(D::Error::custom("Invalid Asset"));
    };

    let mut letters = s.chars();

    let base = vec![
        letters.next().ok_or(D::Error::custom("Invalid Asset"))?,
        letters.next().ok_or(D::Error::custom("Invalid Asset"))?,
        letters.next().ok_or(D::Error::custom("Invalid Asset"))?,
        letters.next().ok_or(D::Error::custom("Invalid Asset"))?,
    ];
    let base: String = base.iter().collect();

    println!("{:?}", base);

    let number: String = letters.collect();
    let number: u32 = number.parse().map_err(D::Error::custom)?;

    println!("{:?}", number);

    let de = StrDeserializer::new(s);

    let asset = Asset::deserialize(base);

    Asset {
        code,
        number,
        f: false,
        t: false,
    }
}

#[derive(Deserialize, Debug)]
struct RawRow {
    #[serde(rename(deserialize = "Entrada/Saída"))]
    inout: InOut,
    
    #[serde(rename(deserialize = "Data"))]
    #[serde(deserialize_with = "date_format")]
    date: NaiveDate,
    
    #[serde(rename(deserialize = "Movimentação"))]
    move_type: MoveType,
    
    #[serde(rename(deserialize = "Produto"))]
    #[serde(deserialize_with = "asset_format")]
    asset: Asset,
    
    #[serde(rename(deserialize = "Instituição"))]
    broker: Broker,
    
    #[serde(rename(deserialize = "Quantidade"))]
    quantity: f64,
    
    #[serde(rename(deserialize = "Preço unitário"))]
    #[serde(deserialize_with = "de_opt_f64")]
    unitary_price: Option<f64>,
    
    #[serde(rename(deserialize = "Valor da Operação"))]
    #[serde(deserialize_with = "de_opt_f64")]
    operation_value: Option<f64>,
}

mod negociation {
    use chrono::NaiveDate;
    use serde::Deserialize;

    use crate::asset_format;
    use crate::date_format;
    use crate::Asset;
    use crate::Broker;
    use crate::InOut;

    #[derive(Deserialize, Debug)]
    pub enum MarketType {
        #[serde(rename(deserialize = "Mercado Fracionário"))]
        Fractional,
    
        #[serde(rename(deserialize = "Mercado à Vista"))]
        Whole,
    }

    #[derive(Deserialize, Debug)]
    pub struct NRow {
        #[serde(rename(deserialize = "Data do Negócio"))]
        #[serde(deserialize_with = "date_format")]
        pub date: NaiveDate,

        #[serde(rename(deserialize = "Tipo de Movimentação"))]
        pub inout: InOut,

        #[serde(rename(deserialize = "Mercado"))]
        pub market_type: MarketType,

        #[serde(rename(deserialize = "Instituição"))]
        pub broker: Broker,

        #[serde(rename(deserialize = "Código de Negociação"))]
        #[serde(deserialize_with = "asset_format")]
        pub asset: Asset,

        #[serde(rename(deserialize = "Quantidade"))]
        pub quantity: f64,

        #[serde(rename(deserialize = "Preço"))]
        pub unitary_price: f64,

        #[serde(rename(deserialize = "Valor"))]
        pub operation_value: f64,
    }
}


#[derive(Debug)]
struct OwnedAsset {
    quantity:    f64,
    total_value: f64,
    mean_value:  f64,
}

impl OwnedAsset {
    fn new() -> Self {
        Self {
            quantity:    0.0,
            total_value: 0.0,
            mean_value:  0.0,
        }
    }

    fn buy(&mut self, quantity: f64, unitary_price: f64, operation_value: f64) {
        self.quantity    += quantity;
        self.total_value += operation_value;
        self.mean_value   = self.total_value / self.quantity;
    }

    fn sell(&mut self, quantity: f64, unitary_price: f64, operation_value: f64) -> f64 {
        let unit_delta = unitary_price - self.mean_value;
        let profit     = unit_delta * quantity;

        self.quantity -= quantity;

        if self.quantity == 0.0 {
            self.total_value = 0.0;
            self.mean_value  = 0.0;
        } else {
            self.total_value -= operation_value;
            self.mean_value   = self.total_value / self.quantity;
        }

        profit
    }


    fn transfer(&mut self, quantity: f64) {
        self.buy(quantity, 0.0, 0.0);
    }

    fn unfold(&mut self, quantity: f64) {
        self.buy(quantity, 0.0, 0.0);
    }
}

fn moves() -> anyhow::Result<()> {

    let mut profit:    f64 = 0.0;
    let mut dividends: f64 = 0.0;

    let mut wallet = HashMap::new();

    let base_dir_entries = fs::read_dir(BASE_DIR)?;

    for entry in base_dir_entries {
        let Ok(entry) = entry else {
            println!("Failed to open dir entry");
            continue;
        };

        let path = entry.path();

        match path.extension() {
            Some(ext) => { if ext != "xlsx" { continue; } }
            None      => {                    continue;   }
        }

        let Ok(mut workbook): Result<Xlsx<_>, _> = open_workbook(&path) else {
            println!("Failed to open xlsx file {:?}", &path);
            continue;
        };

        let Ok(range): Result<Range<_>, _> = workbook.worksheet_range(MOVES) else {
            println!("Failed to open '{MOVES}'");
            continue;
        };

        let Ok(reversed_rows) = RangeDeserializerBuilder::new().from_range::<_, RawRow>(&range) else {
            println!("Failed to read table rows");
            continue;
        };

        let rows: Vec<_> = reversed_rows
            .filter_map(|row| {
                match row {
                    Ok(row) => {
                        Some(row)
                    }
                    Err(err) => {
                        println!("{:?}", err);
                        None
                    }
                }
            })
            .collect();

        for row in rows.into_iter().rev() {
            let RawRow {
                inout,
                date,
                move_type,
                asset,
                broker,
                quantity,
                unitary_price,
                operation_value,
            } = row;

            // println!("Processing {date:?} {asset:?}");

            let owned_asset = wallet
                .entry(asset)
                .or_insert(OwnedAsset::new());

            match (inout, move_type) {
                (InOut::In, MoveType::Atualizacao) => {
                    // Move to marker pro oficial
                }
                (InOut::In, MoveType::BonificacaoEmAtivos) => {}
                (InOut::In, MoveType::CessaoDeDireitos) => {}
                (InOut::In, MoveType::CessaoDeDireitosSolicitada) => {}
                (InOut::In, MoveType::CompraVenda) => {}
                (InOut::In, MoveType::Desdobro) => {
                    owned_asset.unfold(quantity);
                }
                (InOut::In, MoveType::DireitosDeSubscricao) => {}
                (InOut::In, MoveType::DireitosDeSubscricaoExercido) => {}
                (InOut::In, MoveType::DireitosDeSubscricaoNaoExercido) => {}
                (InOut::In, MoveType::DireitoSobrasDeSubscricaoNaoExercido) => {}
                (InOut::In, MoveType::Dividendo) => {
                    if let Some(operation_value) = operation_value {
                        dividends += operation_value;

                        println!("DIV  {:<6} {} {:>9.2}", format!("{}", asset), date, operation_value);
                        println!("Profit:                {:>9.2}", profit);
                        println!("Dividends:             {:>9.2}", dividends);
                        println!("");
                    }
                }
                (InOut::In, MoveType::Emprestimo) => {}
                (InOut::In, MoveType::FracaoEmAtivos) => {}
                (InOut::In, MoveType::Grupamento) => {}
                (InOut::In, MoveType::Incorporacao) => {}
                (InOut::In, MoveType::JurosSobreCapitalProprio) => {
                    if let Some(operation_value) = operation_value {
                        dividends += operation_value;

                        println!("DIV  {:<6} {} {:>9.2}", format!("{}", asset), date, operation_value);
                        println!("Profit:                {:>9.2}", profit);
                        println!("Dividends:             {:>9.2}", dividends);
                        println!("");
                    }
                }
                (InOut::In, MoveType::LeilaoDeFracao) => {}
                (InOut::In, MoveType::ReciboDeSubscricao) => {}
                (InOut::In, MoveType::Reembolso) => {}
                (InOut::In, MoveType::Rendimento) => {
                    if let Some(operation_value) = operation_value {
                        dividends += operation_value;

                        println!("DIV  {:<6} {} {:>9.2}", format!("{}", asset), date, operation_value);
                        println!("Profit:                {:>9.2}", profit);
                        println!("Dividends:             {:>9.2}", dividends);
                        println!("");
                    }
                }
                (InOut::In, MoveType::SolicitacaoDeSubscricao) => {}
                (InOut::In, MoveType::Transferencia) => {
                    owned_asset.transfer(quantity);
                }
                (InOut::In, MoveType::TransferenciaLiquidacao) => {
                    let unitary_price = unitary_price
                        .expect("Missing unitary price");
                    let operation_value = operation_value
                        .expect("Missing operation value");

                    owned_asset.buy(quantity, unitary_price, operation_value);
                }
                (InOut::In, MoveType::Vencimento) => {}

                //

                (InOut::Out, MoveType::Atualizacao) => {}
                (InOut::Out, MoveType::BonificacaoEmAtivos) => {}
                (InOut::Out, MoveType::CessaoDeDireitos) => {}
                (InOut::Out, MoveType::CessaoDeDireitosSolicitada) => {}
                (InOut::Out, MoveType::CompraVenda) => {}
                (InOut::Out, MoveType::Desdobro) => {}
                (InOut::Out, MoveType::DireitosDeSubscricao) => {}
                (InOut::Out, MoveType::DireitosDeSubscricaoExercido) => {}
                (InOut::Out, MoveType::DireitosDeSubscricaoNaoExercido) => {}
                (InOut::Out, MoveType::DireitoSobrasDeSubscricaoNaoExercido) => {}
                (InOut::Out, MoveType::Dividendo) => {}
                (InOut::Out, MoveType::Emprestimo) => {}
                (InOut::Out, MoveType::FracaoEmAtivos) => {}
                (InOut::Out, MoveType::Grupamento) => {}
                (InOut::Out, MoveType::Incorporacao) => {}
                (InOut::Out, MoveType::JurosSobreCapitalProprio) => {}
                (InOut::Out, MoveType::LeilaoDeFracao) => {}
                (InOut::Out, MoveType::ReciboDeSubscricao) => {}
                (InOut::Out, MoveType::Reembolso) => {}
                (InOut::Out, MoveType::Rendimento) => {}
                (InOut::Out, MoveType::SolicitacaoDeSubscricao) => {}
                (InOut::Out, MoveType::Transferencia) => {}
                (InOut::Out, MoveType::TransferenciaLiquidacao) => {
                    let unitary_price = unitary_price
                        .expect("Missing unitary price");
                    let operation_value = operation_value
                        .expect("Missing operation value");

                    let op_profit = owned_asset.sell(quantity, unitary_price, operation_value);

                    profit += op_profit;

                    println!("SOLD {:<6} {} {:>9.2}", format!("{}", asset), date, op_profit);
                    println!("Profit:                {:>9.2}", profit);
                    println!("Dividends:             {:>9.2}", dividends);
                    println!("");
                }
                (InOut::Out, MoveType::Vencimento) => {}
            }
        }
    }

    println!("Profit:                {:>9.2}", profit);
    println!("Dividends:             {:>9.2}", dividends);
    println!("");

    Ok(())
}

fn negociations() -> anyhow::Result<()> {
    

    use negociation::NRow;

    let base_dir_entries = fs::read_dir(BASE_DIR)?;

    for entry in base_dir_entries {
        let Ok(entry) = entry else {
            println!("Failed to open dir entry");
            continue;
        };

        let path = entry.path();

        match path.extension() {
            Some(ext) => { if ext != "xlsx" { continue; } }
            None      => {                    continue;   }
        }

        let Ok(mut workbook): Result<Xlsx<_>, _> = open_workbook(&path) else {
            println!("Failed to open xlsx file {:?}", &path);
            continue;
        };

        let Ok(range): Result<Range<_>, _> = workbook.worksheet_range(NEGOCIATIONS) else {
            println!("Failed to open '{NEGOCIATIONS}'");
            continue;
        };

        let Ok(reversed_rows) = RangeDeserializerBuilder::new().from_range::<_, NRow>(&range) else {
            println!("Failed to read table rows");
            continue;
        };

        let rows: Vec<_> = reversed_rows
            .filter_map(|row| {
                match row {
                    Ok(row) => {
                        Some(row)
                    }
                    Err(err) => {
                        println!("{:?}", err);
                        None
                    }
                }
            })
            .collect();

        for row in rows.into_iter().rev() {
            let NRow {
                asset,
                ..
            } = row;

            println!("Row: {}", asset);
        }
    }

    Ok(())
}

fn main() -> anyhow::Result<()> {
    negociations()
}