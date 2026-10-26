' INV: vbal.5.6.9.3, op.add
' SOURCE: spec ms-vbal 5.6.9.3 Arithmetic operators, effective value types when an operand is Empty
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: result types of mixed operands follow the Learn operator pages, not the type tables of 5.6.9.3
' EXPECT: Empty+Empty=Integer
' EXPECT: Empty+Byte=Byte
' EXPECT: Byte+Empty=Byte
' EXPECT: Empty+Integer=Integer
' EXPECT: Empty+Long=Long
' EXPECT: Long+Empty=Long
' EXPECT: Empty+LongLong=LongLong
' EXPECT: Empty+Single=Single
' EXPECT: Empty+Double=Double
' EXPECT: Empty+Currency=Currency
' EXPECT: Empty+Date=Date
' EXPECT: Empty+Boolean=Integer
' EXPECT: values=0,5,5,7
Public Sub Main()
    Dim e As Variant
    Dim y As Variant: y = CByte(5)
    Dim i As Variant: i = CInt(5)
    Dim l As Variant: l = CLng(5)
    Dim q As Variant: q = CLngLng(5)
    Dim s As Variant: s = CSng(5)
    Dim d As Variant: d = CDbl(5)
    Dim c As Variant: c = CCur(5)
    Dim dt As Variant: dt = CDate(5)
    Dim b As Variant: b = True
    Debug.Print "Empty+Empty=" & TypeName(e + e)
    Debug.Print "Empty+Byte=" & TypeName(e + y)
    Debug.Print "Byte+Empty=" & TypeName(y + e)
    Debug.Print "Empty+Integer=" & TypeName(e + i)
    Debug.Print "Empty+Long=" & TypeName(e + l)
    Debug.Print "Long+Empty=" & TypeName(l + e)
    Debug.Print "Empty+LongLong=" & TypeName(e + q)
    Debug.Print "Empty+Single=" & TypeName(e + s)
    Debug.Print "Empty+Double=" & TypeName(e + d)
    Debug.Print "Empty+Currency=" & TypeName(e + c)
    Debug.Print "Empty+Date=" & TypeName(e + dt)
    Debug.Print "Empty+Boolean=" & TypeName(e + b)
    Debug.Print "values=" & CStr(e + e) & "," & CStr(e + y) & "," & CStr(i + e) & "," & CStr(e + CInt(7))
End Sub
