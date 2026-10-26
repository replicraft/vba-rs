' INV: vbal.5.6.9.3.6, op.intdiv, op.mod
' SOURCE: spec ms-vbal 5.6.9.3.6 \ Operator and Mod Operator, effective value types
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3
' EXPECT: Integer\Integer=Integer
' EXPECT: Byte\Byte=Byte
' EXPECT: Long\Integer=Long
' EXPECT: Integer\Long=Long
' EXPECT: Boolean\Integer=Integer
' EXPECT: Double\Integer=Long
' EXPECT: Currency\Integer=Long
' EXPECT: String\Integer=Long
' EXPECT: Date\Integer=Long
' EXPECT: Single\Single=Long
' EXPECT: LongLong\Integer=LongLong
' EXPECT: Integer\LongLong=LongLong
' EXPECT: Empty\Byte=Integer
' EXPECT: Integer Mod Integer=Integer
' EXPECT: Byte Mod Byte=Byte
' EXPECT: Double Mod Integer=Long
' EXPECT: Long Mod Integer=Long
' EXPECT: LongLong Mod Integer=LongLong
Public Sub Main()
    Dim i As Integer: i = 7
    Dim i2 As Integer: i2 = 2
    Dim y As Byte: y = 7
    Dim y2 As Byte: y2 = 2
    Dim l As Long: l = 7
    Dim q As LongLong: q = 7
    Dim b As Boolean: b = True
    Dim d As Double: d = 7
    Dim s As Single: s = 7
    Dim c As Currency: c = 7
    Dim t As String: t = "7"
    Dim dt As Date: dt = 7
    Dim e As Variant
    Dim v As Variant
    Dim x As Variant
    v = CByte(2)
    x = CByte(7)
    Debug.Print "Integer\Integer=" & TypeName(i \ i2)
    Debug.Print "Byte\Byte=" & TypeName(y \ y2)
    Debug.Print "Long\Integer=" & TypeName(l \ i2)
    Debug.Print "Integer\Long=" & TypeName(i2 \ l)
    Debug.Print "Boolean\Integer=" & TypeName(b \ i2)
    Debug.Print "Double\Integer=" & TypeName(d \ i2)
    Debug.Print "Currency\Integer=" & TypeName(c \ i2)
    Debug.Print "String\Integer=" & TypeName(t \ i2)
    Debug.Print "Date\Integer=" & TypeName(dt \ i2)
    Debug.Print "Single\Single=" & TypeName(s \ s)
    Debug.Print "LongLong\Integer=" & TypeName(q \ i2)
    Debug.Print "Integer\LongLong=" & TypeName(i2 \ q)
    Debug.Print "Empty\Byte=" & TypeName(e \ v)
    Debug.Print "Integer Mod Integer=" & TypeName(i Mod i2)
    Debug.Print "Byte Mod Byte=" & TypeName(y Mod y2)
    Debug.Print "Double Mod Integer=" & TypeName(d Mod i2)
    Debug.Print "Long Mod Integer=" & TypeName(l Mod i2)
    Debug.Print "LongLong Mod Integer=" & TypeName(q Mod i2)
End Sub
