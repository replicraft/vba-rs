' INV: vbal.6.1.2.3.1.10, func.clngptr
' SOURCE: spec ms-vbal 6.1.2.3.1.10 CLngPtr coercion
' EXPECT: r25=2
' EXPECT: r35=4
' EXPECT: str=77
' EXPECT: true=-1
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Debug.Print "r25=" & CStr(CLngPtr(2.5))
    Debug.Print "r35=" & CStr(CLngPtr(3.5))
    Debug.Print "str=" & CStr(CLngPtr("77"))
    Debug.Print "true=" & CStr(CLngPtr(True))
    On Error Resume Next
    Err.Clear: v = CLngPtr(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CLngPtr("zz"): Debug.Print "text=" & Err.Number
End Sub
