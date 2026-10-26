' INV: vbal.6.1.2.11.1.36, func.strcomp
' SOURCE: spec ms-vbal 6.1.2.11.1.36 StrComp with a Null Compare argument is an error
' EXPECT: err=True
Public Sub Main()
    Dim r As Variant
    On Error Resume Next
    r = StrComp("a", "b", Null)
    Debug.Print "err=" & CStr(Err.Number <> 0)
End Sub
