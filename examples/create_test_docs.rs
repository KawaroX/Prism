use prism::DocxBuilder;
use prism::docx::{Document, Paragraph, Run, RunContent, RunProperties};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建基础文档
    let mut base_doc = Document::new();

    // 添加标题
    add_heading(&mut base_doc, "研究论文", 1);

    // 添加章节1
    add_heading(&mut base_doc, "1. 引言", 2);
    add_paragraph(
        &mut base_doc,
        "本研究旨在探讨文档版本控制系统在学术写作中的应用。团队协作写作是现代学术研究中的常见实践，然而对于文科生而言，传统的版本控制工具往往存在使用门槛高的问题。",
    );

    // 添加章节2
    add_heading(&mut base_doc, "2. 研究方法", 2);
    add_paragraph(
        &mut base_doc,
        "本研究采用文献综述和用户调查相结合的方法。我们收集了50名文科专业学生的使用反馈，并分析了现有工具的优缺点。",
    );

    // 保存基础文档
    DocxBuilder::build(&base_doc, "base_document.docx")?;

    // 创建修改版本1（修改引言，添加新章节）
    let mut doc1 = base_doc.clone();

    // 修改引言段落（第3个段落，索引2）
    doc1.body.paragraphs[2] = create_paragraph(
        "本研究旨在探讨文档版本控制系统在学术写作中的应用和优化。团队协作写作对于现代学术研究至关重要，然而传统的版本控制工具对于文科生而言往往存在使用门槛过高的问题。我们的研究针对这一问题提出了新的解决方案。",
    );

    // 添加新章节
    add_heading(&mut doc1, "3. 文献综述", 2);
    add_paragraph(
        &mut doc1,
        "现有研究表明，多数文科生在团队协作过程中依赖于简单的文件共享工具，如电子邮件和云存储服务，这些方式在处理文档合并时效率较低。",
    );

    // 保存修改版本1
    DocxBuilder::build(&doc1, "modified_document1.docx")?;

    // 创建修改版本2（修改研究方法，保持引言不变）
    let mut doc2 = base_doc.clone();

    // 修改研究方法段落（第5个段落，索引4）
    doc2.body.paragraphs[4] = create_paragraph(
        "本研究采用混合研究方法论，结合定性和定量分析。我们收集了来自三所大学的100名文科专业学生的使用数据和深度访谈，并通过统计学方法分析了他们在团队写作中遇到的主要挑战。",
    );

    // 添加新章节，但内容与doc1不同
    add_heading(&mut doc2, "3. 研究假设", 2);
    add_paragraph(
        &mut doc2,
        "基于前期调研，我们提出以下研究假设：1）简化的版本控制接口将显著提高文科生的协作效率；2）集成的文件格式转换功能能减少团队成员间的技术障碍。",
    );

    // 保存修改版本2
    DocxBuilder::build(&doc2, "modified_document2.docx")?;

    println!("已成功创建三个测试文档：");
    println!("1. base_document.docx - 基础版本");
    println!("2. modified_document1.docx - 修改了引言并添加文献综述");
    println!("3. modified_document2.docx - 修改了研究方法并添加研究假设");

    Ok(())
}

// 辅助函数：添加标题
fn add_heading(doc: &mut Document, text: &str, level: usize) {
    let style_id = format!("Heading{}", level);

    let heading = Paragraph {
        id: None,
        style_id: Some(style_id),
        properties: Default::default(),
        runs: vec![Run {
            properties: {
                let mut props = RunProperties::default();
                props.bold = true;
                props.size = Some(28 - (level as u32 * 2)); // 根据级别调整大小
                props
            },
            contents: vec![RunContent::Text(text.to_string())],
        }],
    };

    doc.body.paragraphs.push(heading);
}

// 辅助函数：添加普通段落
fn add_paragraph(doc: &mut Document, text: &str) {
    let paragraph = create_paragraph(text);
    doc.body.paragraphs.push(paragraph);
}

// 辅助函数：创建普通段落
fn create_paragraph(text: &str) -> Paragraph {
    Paragraph {
        id: None,
        style_id: Some("Normal".to_string()),
        properties: Default::default(),
        runs: vec![Run {
            properties: Default::default(),
            contents: vec![RunContent::Text(text.to_string())],
        }],
    }
}
