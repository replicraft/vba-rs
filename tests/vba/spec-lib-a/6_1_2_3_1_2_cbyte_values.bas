' INV: vbal.6.1.2.3.1.2, func.cbyte
' SOURCE: spec ms-vbal 6.1.2.3.1.2 CByte coercion and banker's rounding
' EXPECT: r25=2
' EXPECT: r35=4
' EXPECT: r05=0
' EXPECT: r15=2
' EXPECT: max=255
' EXPECT: str=12
' EXPECT: strExp=100
' EXPECT: empty=0
' EXPECT: type=Byte
Public Sub Main()
    Dim e As Variant
    Debug.Print "r25=" & CStr(CByte(2.5))
    Debug.Print "r35=" & CStr(CByte(3.5))
    Debug.Print "r05=" & CStr(CByte(0.5))
    Debug.Print "r15=" & CStr(CByte(1.5))
    Debug.Print "max=" & CStr(CByte(255))
    Debug.Print "str=" & CStr(CByte("12"))
    Debug.Print "strExp=" & CStr(CByte("1E2"))
    Debug.Print "empty=" & CStr(CByte(e))
    Debug.Print "type=" & TypeName(CByte(1))
End Sub
