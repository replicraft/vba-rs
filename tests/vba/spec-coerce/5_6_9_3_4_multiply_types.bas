' INV: vbal.5.6.9.3.4, op.mul
' SOURCE: spec ms-vbal 5.6.9.3.4 * Operator, exceptions: Currency with Single/Double/String and anything with Date give Double
' XFAIL: wrong pending
' EXPECT: Currency*Integer=Currency
' EXPECT: Integer*Currency=Currency
' EXPECT: Currency*Currency=Currency
' EXPECT: Currency*Long=Currency
' EXPECT: Currency*Single=Double
' EXPECT: Single*Currency=Double
' EXPECT: Currency*Double=Double
' EXPECT: Double*Currency=Double
' EXPECT: Currency*String=Double
' EXPECT: String*Currency=Double
' EXPECT: Date*Integer=Double
' EXPECT: Integer*Date=Double
' EXPECT: Date*Date=Double
' EXPECT: Byte*Byte=Byte
' EXPECT: Integer*Integer=Integer
' EXPECT: Integer*Long=Long
' EXPECT: Single*Single=Single
' EXPECT: Single*Long=Double
' EXPECT: Decimal*Integer=Decimal
' EXPECT: 7.5
' EXPECT: 12
Public Sub Main()
    Dim c As Currency: c = 2
    Dim i As Integer: i = 3
    Dim l As Long: l = 3
    Dim s As Single: s = 2
    Dim d As Double: d = 2
    Dim t As String: t = "2"
    Dim dt As Date: dt = 2
    Dim y As Byte: y = 2
    Dim dec As Variant: dec = CDec(2)
    Debug.Print "Currency*Integer=" & TypeName(c * i)
    Debug.Print "Integer*Currency=" & TypeName(i * c)
    Debug.Print "Currency*Currency=" & TypeName(c * c)
    Debug.Print "Currency*Long=" & TypeName(c * l)
    Debug.Print "Currency*Single=" & TypeName(c * s)
    Debug.Print "Single*Currency=" & TypeName(s * c)
    Debug.Print "Currency*Double=" & TypeName(c * d)
    Debug.Print "Double*Currency=" & TypeName(d * c)
    Debug.Print "Currency*String=" & TypeName(c * t)
    Debug.Print "String*Currency=" & TypeName(t * c)
    Debug.Print "Date*Integer=" & TypeName(dt * i)
    Debug.Print "Integer*Date=" & TypeName(i * dt)
    Debug.Print "Date*Date=" & TypeName(dt * dt)
    Debug.Print "Byte*Byte=" & TypeName(y * y)
    Debug.Print "Integer*Integer=" & TypeName(i * i)
    Debug.Print "Integer*Long=" & TypeName(i * l)
    Debug.Print "Single*Single=" & TypeName(s * s)
    Debug.Print "Single*Long=" & TypeName(s * l)
    Debug.Print "Decimal*Integer=" & TypeName(dec * i)
    Debug.Print CStr(2.5 * 3)
    Debug.Print CStr(y * i * y)
End Sub
