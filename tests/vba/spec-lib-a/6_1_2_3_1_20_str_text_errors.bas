' INV: vbal.6.1.2.3.1.20, func.str
' SOURCE: spec ms-vbal 6.1.2.3.1.20 Str of a non-numeric String goes through Double coercion
' EXPECT: text=13
' EXPECT: dollarNull=94
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = Str("abc"): Debug.Print "text=" & Err.Number
    Err.Clear: v = Str$(Null): Debug.Print "dollarNull=" & Err.Number
End Sub
