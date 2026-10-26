' INV: vbal.5.5.1.2.12, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.12 Let-coercion to Variant, the Variant receives a copy of the source value with its own value type
' EXPECT: Integer
' EXPECT: Long
' EXPECT: Double
' EXPECT: Single
' EXPECT: String
' EXPECT: Boolean
' EXPECT: Date
' EXPECT: Currency
' EXPECT: Byte
' EXPECT: Decimal
' EXPECT: Empty
' EXPECT: Null
' EXPECT: String
Public Sub Main()
    Dim v As Variant
    Dim i As Integer: i = 1
    Dim l As Long: l = 1
    Dim d As Double: d = 1
    Dim s As Single: s = 1
    Dim t As String: t = "x"
    Dim b As Boolean: b = True
    Dim dt As Date: dt = 1
    Dim c As Currency: c = 1
    Dim y As Byte: y = 1
    Dim f As String * 2: f = "ab"
    v = i: Debug.Print TypeName(v)
    v = l: Debug.Print TypeName(v)
    v = d: Debug.Print TypeName(v)
    v = s: Debug.Print TypeName(v)
    v = t: Debug.Print TypeName(v)
    v = b: Debug.Print TypeName(v)
    v = dt: Debug.Print TypeName(v)
    v = c: Debug.Print TypeName(v)
    v = y: Debug.Print TypeName(v)
    v = CDec(1): Debug.Print TypeName(v)
    Dim e As Variant
    v = e: Debug.Print TypeName(v)
    v = Null: Debug.Print TypeName(v)
    v = f: Debug.Print TypeName(v)
End Sub
