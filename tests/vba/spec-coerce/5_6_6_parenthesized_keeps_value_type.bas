' INV: vbal.5.6.6, expr.paren
' SOURCE: spec ms-vbal 5.6.6 Parenthesized expressions, the value type of a parenthesized expression is that of the enclosed expression
' EXPECT: Byte
' EXPECT: Currency
' EXPECT: Date
' EXPECT: Boolean
' EXPECT: String
' EXPECT: Single
' EXPECT: Empty
' EXPECT: Null
Public Sub Main()
    Dim v As Variant
    Dim c As Currency: c = 1
    Dim d As Date: d = 1
    Dim s As Single: s = 1
    Debug.Print TypeName((CByte(3)))
    Debug.Print TypeName((c))
    Debug.Print TypeName(((d)))
    Debug.Print TypeName((True))
    Debug.Print TypeName(("x"))
    Debug.Print TypeName((s))
    Debug.Print TypeName((v))
    Debug.Print TypeName((Null))
End Sub
