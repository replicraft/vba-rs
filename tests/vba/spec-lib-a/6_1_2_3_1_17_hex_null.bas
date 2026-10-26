' INV: vbal.6.1.2.3.1.17, func.hex
' SOURCE: spec ms-vbal 6.1.2.3.1.17 Hex(Null) is Null, Hex$(Null) raises 94
' EXPECT: hexNull=True
' EXPECT: dollar=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Debug.Print "hexNull=" & CStr(IsNull(Hex(Null)))
    On Error Resume Next
    Err.Clear: v = Hex$(Null): Debug.Print "dollar=" & Err.Number
    Err.Clear: v = Hex("zz"): Debug.Print "text=" & Err.Number
End Sub
