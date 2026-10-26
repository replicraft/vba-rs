' INV: vbal.6.1.2.8.1.16, func.switch
' SOURCE: spec ms-vbal 6.1.2.8.1.16 Switch: improperly paired arguments raise a run-time error
' EXPECT: err
Public Sub Main()
    On Error Resume Next
    Dim v As Variant
    v = Switch(False, "a", True)
    If Err.Number <> 0 Then Debug.Print "err" Else Debug.Print "noerr"
End Sub
