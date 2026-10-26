' INV: vbal.6.1.2.3.1.20, func.str
' SOURCE: spec ms-vbal 6.1.2.3.1.20 Str leading space for non-negative numbers
' EXPECT: five=[ 5]
' EXPECT: neg=[-5]
' EXPECT: zero=[ 0]
' EXPECT: dbl=[ 1.5]
' EXPECT: negdbl=[-1.5]
' EXPECT: cur=[ 2]
' EXPECT: byte=[ 7]
' EXPECT: long=[ 70000]
' EXPECT: strnum=[ 5]
' EXPECT: empty=[ 0]
' EXPECT: true=[-1]
' EXPECT: false=[ 0]
' EXPECT: dollar=[ 5]
' EXPECT: dollarType=String
Public Sub Main()
    Dim e As Variant
    Debug.Print "five=[" & Str(5) & "]"
    Debug.Print "neg=[" & Str(-5) & "]"
    Debug.Print "zero=[" & Str(0) & "]"
    Debug.Print "dbl=[" & Str(1.5) & "]"
    Debug.Print "negdbl=[" & Str(-1.5) & "]"
    Debug.Print "cur=[" & Str(CCur(2)) & "]"
    Debug.Print "byte=[" & Str(CByte(7)) & "]"
    Debug.Print "long=[" & Str(70000) & "]"
    Debug.Print "strnum=[" & Str("5") & "]"
    Debug.Print "empty=[" & Str(e) & "]"
    Debug.Print "true=[" & Str(True) & "]"
    Debug.Print "false=[" & Str(False) & "]"
    Debug.Print "dollar=[" & Str$(5) & "]"
    Debug.Print "dollarType=" & TypeName(Str$(5))
End Sub
