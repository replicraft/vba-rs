' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, Single/Currency/Decimal sources use Banker's rounding to integral types
' EXPECT: single 2.5=2
' EXPECT: single 3.5=4
' EXPECT: single -2.5=-2
' EXPECT: cur 2.5=2
' EXPECT: cur 3.5=4
' EXPECT: cur 0.5=0
' EXPECT: cur -2.5=-2
' EXPECT: cur 2.5001=3
' EXPECT: cur 2.4999=2
' EXPECT: dec 2.5=2
' EXPECT: dec 3.5=4
' EXPECT: dec -2.5=-2
' EXPECT: dec 0.5=0
' EXPECT: dec 1.5=2
' EXPECT: dec 2.5000000000000001=3
Public Sub Main()
    Dim i As Integer
    Dim l As Long
    Dim s As Single
    Dim c As Currency
    Dim v As Variant
    s = 2.5: i = s: Debug.Print "single 2.5=" & CStr(i)
    s = 3.5: i = s: Debug.Print "single 3.5=" & CStr(i)
    s = -2.5: i = s: Debug.Print "single -2.5=" & CStr(i)
    c = 2.5: i = c: Debug.Print "cur 2.5=" & CStr(i)
    c = 3.5: i = c: Debug.Print "cur 3.5=" & CStr(i)
    c = 0.5: i = c: Debug.Print "cur 0.5=" & CStr(i)
    c = -2.5: l = c: Debug.Print "cur -2.5=" & CStr(l)
    c = 2.5001: i = c: Debug.Print "cur 2.5001=" & CStr(i)
    c = 2.4999: i = c: Debug.Print "cur 2.4999=" & CStr(i)
    v = CDec("2.5"): i = v: Debug.Print "dec 2.5=" & CStr(i)
    v = CDec("3.5"): i = v: Debug.Print "dec 3.5=" & CStr(i)
    v = CDec("-2.5"): l = v: Debug.Print "dec -2.5=" & CStr(l)
    v = CDec("0.5"): i = v: Debug.Print "dec 0.5=" & CStr(i)
    v = CDec("1.5"): i = v: Debug.Print "dec 1.5=" & CStr(i)
    v = CDec("2.5000000000000001"): i = v: Debug.Print "dec 2.5000000000000001=" & CStr(i)
End Sub
