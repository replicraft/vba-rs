' INV: vbal.6.1.2.3.1.7, func.cint
' SOURCE: spec ms-vbal 6.1.2.3.1.7 CInt coercion with banker's rounding and overflow
' EXPECT: r25=2
' EXPECT: r35=4
' EXPECT: rn25=-2
' EXPECT: rn35=-4
' EXPECT: max=32767
' EXPECT: min=-32768
' EXPECT: round=32767
' EXPECT: true=-1
' EXPECT: str=12
' EXPECT: strws=12
' EXPECT: empty=0
' EXPECT: type=Integer
' EXPECT: over=6
' EXPECT: under=6
' EXPECT: roundOver=6
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Dim e As Variant
    Debug.Print "r25=" & CStr(CInt(2.5))
    Debug.Print "r35=" & CStr(CInt(3.5))
    Debug.Print "rn25=" & CStr(CInt(-2.5))
    Debug.Print "rn35=" & CStr(CInt(-3.5))
    Debug.Print "max=" & CStr(CInt(32767))
    Debug.Print "min=" & CStr(CInt(-32768))
    Debug.Print "round=" & CStr(CInt(32767.4))
    Debug.Print "true=" & CStr(CInt(True))
    Debug.Print "str=" & CStr(CInt("12"))
    Debug.Print "strws=" & CStr(CInt("  12 "))
    Debug.Print "empty=" & CStr(CInt(e))
    Debug.Print "type=" & TypeName(CInt(1))
    On Error Resume Next
    Err.Clear: v = CInt(32768): Debug.Print "over=" & Err.Number
    Err.Clear: v = CInt(-32769): Debug.Print "under=" & Err.Number
    Err.Clear: v = CInt(32767.5): Debug.Print "roundOver=" & Err.Number
    Err.Clear: v = CInt(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CInt("x1"): Debug.Print "text=" & Err.Number
End Sub
