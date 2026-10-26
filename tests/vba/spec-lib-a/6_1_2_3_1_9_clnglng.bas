' INV: vbal.6.1.2.3.1.9, func.clnglng
' SOURCE: spec ms-vbal 6.1.2.3.1.9 CLngLng coercion
' EXPECT: r25=2
' EXPECT: r35=4
' EXPECT: big=1099511627776
' EXPECT: true=-1
' EXPECT: str=123456789012
' EXPECT: type=LongLong
' EXPECT: over=6
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Debug.Print "r25=" & CStr(CLngLng(2.5))
    Debug.Print "r35=" & CStr(CLngLng(3.5))
    Debug.Print "big=" & CStr(CLngLng(1099511627776#))
    Debug.Print "true=" & CStr(CLngLng(True))
    Debug.Print "str=" & CStr(CLngLng("123456789012"))
    Debug.Print "type=" & TypeName(CLngLng(1))
    On Error Resume Next
    Err.Clear: v = CLngLng(1E+19): Debug.Print "over=" & Err.Number
    Err.Clear: v = CLngLng(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CLngLng("zz"): Debug.Print "text=" & Err.Number
End Sub
