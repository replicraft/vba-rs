' INV: vbal.6.1.2.3.1.2, func.cbyte
' SOURCE: spec ms-vbal 6.1.2.3.1.2 CByte error cases
' EXPECT: over256=6
' EXPECT: neg1=6
' EXPECT: round2555=6
' EXPECT: str256=6
' EXPECT: strText=13
' EXPECT: null=94
' EXPECT: ok2554=0
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = CByte(256): Debug.Print "over256=" & Err.Number
    Err.Clear: v = CByte(-1): Debug.Print "neg1=" & Err.Number
    Err.Clear: v = CByte(255.5): Debug.Print "round2555=" & Err.Number
    Err.Clear: v = CByte("256"): Debug.Print "str256=" & Err.Number
    Err.Clear: v = CByte("x"): Debug.Print "strText=" & Err.Number
    Err.Clear: v = CByte(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CByte(255.4): Debug.Print "ok2554=" & Err.Number
End Sub
