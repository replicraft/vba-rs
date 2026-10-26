' INV: vbal.6.1.2.3.1.19, func.oct
' SOURCE: spec ms-vbal 6.1.2.3.1.19 Oct(Null) is Null, Oct$(Null) raises 94
' EXPECT: octNull=True
' EXPECT: dollar=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Debug.Print "octNull=" & CStr(IsNull(Oct(Null)))
    On Error Resume Next
    Err.Clear: v = Oct$(Null): Debug.Print "dollar=" & Err.Number
    Err.Clear: v = Oct("zz"): Debug.Print "text=" & Err.Number
End Sub
