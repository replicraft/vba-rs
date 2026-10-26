' INV: vbal.6.1.2.3.1.17, func.hex
' SOURCE: spec ms-vbal 6.1.2.3.1.17 Hex positive values, Empty, coercion to Long
' EXPECT: ff=FF
' EXPECT: zero=0
' EXPECT: one=1
' EXPECT: x8000=8000
' EXPECT: max=7FFFFFFF
' EXPECT: empty=0
' EXPECT: r25=2
' EXPECT: r35=4
' EXPECT: r2555=100
' EXPECT: str=FF
' EXPECT: byte=FF
' EXPECT: dollarEmpty=0
' EXPECT: longlong=100000000
' EXPECT: type=String
' EXPECT: dollarType=String
Public Sub Main()
    Dim e As Variant
    Debug.Print "ff=" & Hex(255)
    Debug.Print "zero=" & Hex(0)
    Debug.Print "one=" & Hex(1)
    Debug.Print "x8000=" & Hex(32768)
    Debug.Print "max=" & Hex(2147483647)
    Debug.Print "empty=" & Hex(e)
    Debug.Print "r25=" & Hex(2.5)
    Debug.Print "r35=" & Hex(3.5)
    Debug.Print "r2555=" & Hex(255.5)
    Debug.Print "str=" & Hex("255")
    Debug.Print "byte=" & Hex(CByte(255))
    Debug.Print "dollarEmpty=" & Hex$(e)
    Debug.Print "longlong=" & Hex(CLngLng(4294967296#))
    Debug.Print "type=" & TypeName(Hex(1))
    Debug.Print "dollarType=" & TypeName(Hex$(1))
End Sub
