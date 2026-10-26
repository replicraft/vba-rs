' INV: vbal.5.6.9.3.5, op.div
' SOURCE: spec ms-vbal 5.6.9.3.5 / Operator, integral operands and Double/String/Currency/Date operands give Double; Single and Decimal keep their type
' XFAIL: wrong pending
' EXPECT: Integer/Integer=Double 3.5
' EXPECT: Byte/Byte=Double 0.5
' EXPECT: Long/Long=Double 2
' EXPECT: Boolean/Integer=Double -0.5
' EXPECT: Currency/Currency=Double 2.5
' EXPECT: Currency/Integer=Double 2.5
' EXPECT: Integer/Currency=Double 0.4
' EXPECT: String/Integer=Double 2.5
' EXPECT: Integer/String=Double 0.4
' EXPECT: Double/Integer=Double 2.5
' EXPECT: Date/Integer=Double 2.5
' EXPECT: Single/Single=Single 2.5
' EXPECT: Single/Integer=Single 2.5
' EXPECT: Integer/Single=Single 0.4
' EXPECT: Decimal/Decimal=Decimal 3.5
Public Sub Main()
    Dim i7 As Integer: i7 = 7
    Dim i2 As Integer: i2 = 2
    Dim y1 As Byte: y1 = 1
    Dim y2 As Byte: y2 = 2
    Dim l4 As Long: l4 = 4
    Dim l2 As Long: l2 = 2
    Dim bT As Boolean: bT = True
    Dim c5 As Currency: c5 = 5
    Dim c2 As Currency: c2 = 2
    Dim t5 As String: t5 = "5"
    Dim d5 As Double: d5 = 5
    Dim dt5 As Date: dt5 = 5
    Dim s5 As Single: s5 = 5
    Dim s2 As Single: s2 = 2
    Dim dec7 As Variant: dec7 = CDec(7)
    Dim dec2 As Variant: dec2 = CDec(2)
    Debug.Print "Integer/Integer=" & TypeName(i7 / i2) & " " & CStr(i7 / i2)
    Debug.Print "Byte/Byte=" & TypeName(y1 / y2) & " " & CStr(y1 / y2)
    Debug.Print "Long/Long=" & TypeName(l4 / l2) & " " & CStr(l4 / l2)
    Debug.Print "Boolean/Integer=" & TypeName(bT / i2) & " " & CStr(bT / i2)
    Debug.Print "Currency/Currency=" & TypeName(c5 / c2) & " " & CStr(c5 / c2)
    Debug.Print "Currency/Integer=" & TypeName(c5 / i2) & " " & CStr(c5 / i2)
    Debug.Print "Integer/Currency=" & TypeName(i2 / c5) & " " & CStr(i2 / c5)
    Debug.Print "String/Integer=" & TypeName(t5 / i2) & " " & CStr(t5 / i2)
    Debug.Print "Integer/String=" & TypeName(i2 / t5) & " " & CStr(i2 / t5)
    Debug.Print "Double/Integer=" & TypeName(d5 / i2) & " " & CStr(d5 / i2)
    Debug.Print "Date/Integer=" & TypeName(dt5 / i2) & " " & CStr(dt5 / i2)
    Debug.Print "Single/Single=" & TypeName(s5 / s2) & " " & CStr(s5 / s2)
    Debug.Print "Single/Integer=" & TypeName(s5 / i2) & " " & CStr(s5 / i2)
    Debug.Print "Integer/Single=" & TypeName(i2 / s5) & " " & CStr(i2 / s5)
    Debug.Print "Decimal/Decimal=" & TypeName(dec7 / dec2) & " " & CStr(dec7 / dec2)
End Sub
