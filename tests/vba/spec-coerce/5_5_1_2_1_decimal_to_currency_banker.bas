' INV: vbal.5.5.1.2.1, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.1 Let-coercion between numeric types, fixed-point to fixed-point rounds to nearest representable using Banker's rounding
' EXPECT: 0.00005=0
' EXPECT: 0.00015=0.0002
' EXPECT: 0.00025=0.0002
' EXPECT: 0.00035=0.0004
' EXPECT: 0.00016=0.0002
' EXPECT: 0.00014=0.0001
' EXPECT: -0.00015=-0.0002
' EXPECT: -0.00025=-0.0002
Public Sub Main()
    Dim c As Currency
    Dim v As Variant
    v = CDec("0.00005"): c = v: Debug.Print "0.00005=" & CStr(c)
    v = CDec("0.00015"): c = v: Debug.Print "0.00015=" & CStr(c)
    v = CDec("0.00025"): c = v: Debug.Print "0.00025=" & CStr(c)
    v = CDec("0.00035"): c = v: Debug.Print "0.00035=" & CStr(c)
    v = CDec("0.00016"): c = v: Debug.Print "0.00016=" & CStr(c)
    v = CDec("0.00014"): c = v: Debug.Print "0.00014=" & CStr(c)
    v = CDec("-0.00015"): c = v: Debug.Print "-0.00015=" & CStr(c)
    v = CDec("-0.00025"): c = v: Debug.Print "-0.00025=" & CStr(c)
End Sub
