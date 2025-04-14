use prism::diff::{DiffOptions, DiffRenderer, DocumentDiffer, DocxDiffer};
use prism::docx::DocxParser;
use prism::error::Result;

fn main() -> Result<()> {
    // 解析三个文档
    println!("正在解析文档...");
    let base_doc = DocxParser::parse_file("base_document.docx")?;
    let doc1 = DocxParser::parse_file("modified_document1.docx")?;
    let doc2 = DocxParser::parse_file("modified_document2.docx")?;

    // 创建差异检测器和渲染器
    let differ = DocxDiffer::new();
    let renderer = DiffRenderer::new();
    let options = DiffOptions::default();

    // 比较基础文档和修改版本1
    println!("\n=== 基础文档与修改版本1的差异 ===");
    let diff1 = differ.diff(&base_doc, &doc1, &options)?;

    println!("找到 {} 处差异:", diff1.operations.len());
    let diff_text1 = renderer.render_as_text(&diff1)?;
    println!("{}", diff_text1);

    // 比较基础文档和修改版本2
    println!("\n=== 基础文档与修改版本2的差异 ===");
    let diff2 = differ.diff(&base_doc, &doc2, &options)?;

    println!("找到 {} 处差异:", diff2.operations.len());
    let diff_text2 = renderer.render_as_text(&diff2)?;
    println!("{}", diff_text2);

    // 检查两个修改版本之间是否存在冲突
    println!("\n=== 检查修改版本之间的潜在冲突 ===");
    let are_conflicting = !differ.are_equal(&doc1, &doc2, &options)?;

    if are_conflicting {
        println!("两个修改版本之间存在差异，可能需要手动合并。");

        // 显示两个修改版本之间的差异
        let diff_between = differ.diff(&doc1, &doc2, &options)?;
        let diff_text_between = renderer.render_as_text(&diff_between)?;
        println!("\n修改版本1和修改版本2之间的差异:");
        println!("{}", diff_text_between);
    } else {
        println!("两个修改版本之间没有冲突，可以自动合并。");
    }

    Ok(())
}
