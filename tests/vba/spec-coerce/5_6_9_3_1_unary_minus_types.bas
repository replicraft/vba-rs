' INV: vbal.5.6.9.3.1, op.neg
' SOURCE: spec ms-vbal 5.6.9.3.1 Unary - operator, effective value types (Byte becomes Integer, Boolean/Empty become Integer, String becomes Double)
' EXPECT: Byte=Integer -5
' EXPECT: Boolean=Integer 1
' EXPECT: Integer=Integer -5
' EXPECT: Long=Long -5
' EXPECT: LongLong=LongLong -5
' EXPECT: Single=Single -5
' EXPECT: Double=Double -5
' EXPECT: String=Double -5
' EXPECT: Currency=Currency -5
' EXPECT: Decimal=Decimal -5
' EXPECT: Empty=Integer 0
' EXPECT: Date=Date -5
Public Sub Main()
    Dim y As Byte: y = 5
    Dim b As Boolean: b = True
    Dim i As Integer: i = 5
    Dim l As Long: l = 5
    Dim q As LongLong: q = 5
    Dim s As Single: s = 5
    Dim d As Double: d = 5
    Dim t As String: t = "5"
    Dim c As Currency: c = 5
    Dim dec As Variant: dec = CDec(5)
    Dim e As Variant
    Dim dt As Date: dt = 5
    Debug.Print "Byte=" & TypeName(-y) & " " & CStr(-y)
    Debug.Print "Boolean=" & TypeName(-b) & " " & CStr(-b)
    Debug.Print "Integer=" & TypeName(-i) & " " & CStr(-i)
    Debug.Print "Long=" & TypeName(-l) & " " & CStr(-l)
    Debug.Print "LongLong=" & TypeName(-q) & " " & CStr(-q)
    Debug.Print "Single=" & TypeName(-s) & " " & CStr(-s)
    Debug.Print "Double=" & TypeName(-d) & " " & CStr(-d)
    Debug.Print "String=" & TypeName(-t) & " " & CStr(-t)
    Debug.Print "Currency=" & TypeName(-c) & " " & CStr(-c)
    Debug.Print "Decimal=" & TypeName(-dec) & " " & CStr(-dec)
    Debug.Print "Empty=" & TypeName(-e) & " " & CStr(-e)
    Debug.Print "Date=" & TypeName(-dt) & " " & CStr(CDbl(-dt))
End Sub
