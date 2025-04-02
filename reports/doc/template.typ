
#let cover(
  title: "",
  subtitle: none,
  author: "",
  affiliation: none,
  year: none,
  class: none,
  date: datetime.today().display(),
  student_no: none,
  logo: none,
  alpha: 20%,
  color-words: (),
  body,
) = {
  // 设置文档属性，不会在 PDF 中显示，但是会嵌入到文档的元数据中
  set document(author: author, title: title)

  // 保存字体
  let zh-font = "SimSun"
  let en-font = "Times New Roman"
  // let title-font = "SimSun"
  let title-font = "FZKai-Z03"

  // 设置颜色
  let microsoft_blue = "0070C0"
  let black = "000000"
  let primary-color = rgb(microsoft_blue) // alpha = 100%
  let secondary-color = rgb(black) // alpha = 100%

  // 高亮重要单词
  show regex(if color-words.len() == 0 { "$ " } else { color-words.join("|") }): text.with(fill: primary-color)

  // 自定义内联代码样式（添加背景虚化）
  show raw.where(block: false) : it => h(0.5em) + box(fill: primary-color.lighten(90%), outset: 0.2em, it) + h(0.5em)

  // 设置正文字体
  set text(lang: "zh", font: zh-font, 12pt)
  set text(lang: "en", font: en-font, 12pt)
 
  // --------------- 标题设置 -----------------
  show heading: set text(font: title-font, fill: primary-color, weight: "bold")
  // 一级标题
  show heading.where(level: 1): set text(font: title-font, fill: rgb(black), size: 24pt)
  show heading.where(level:1): it => it + v(0.5em)
  
 
  // 设置链接样式
  show link: it => underline(text(fill: primary-color, it))

  // 设置编号列表样式
  set enum(indent: 1em, numbering: n => [#text(fill: secondary-color, numbering("1.", n))])

  // 设置无序列表样式
  set list(indent: 1em, marker: n => [#text(fill: secondary-color, "•")])

  // --------------标题页--------------
  // 如果给出logo，则将其放置在中间
  // if logo != none {
  //   set image(width: 90%)
  //   set align(center+top)
  //   block(logo)
  // }
  
  // // 添加标题
  // align(center, text(font: title-font, 3em, weight: 700, title))
  // v(2em, weak: true)

  // // 添加副标题
  // if subtitle != none {
  // align(center, text(font: title-font, 2em, weight: 700, subtitle))
  // }

  // // 添加作者和其它信息
  // // 居中对齐后左端对齐
  // align(center + bottom, block([
  //   #set align(left)
  //   #set text(size: 24pt, font: title-font)
  //   #set underline(offset: .15em, stroke: .05em, evade: false)
    
  //   #if author != "" {[姓名： #underline(author)\ ]}
  //   // #if class != none {[班级： #underline(class)\ ]}
  //   // #if student_no != none {[学号： #underline(student_no)\ ]}
  //   #if date != none {[日期： #underline(date)\ ]}
  // ]
  // ))
  // pagebreak() // 换页

  // 生成目录
  // outline()
  // pagebreak() // 换页

  // --------------页眉页脚--------------
  set page(
    numbering: "1", 
    number-align: center, 
    )

  body
}
