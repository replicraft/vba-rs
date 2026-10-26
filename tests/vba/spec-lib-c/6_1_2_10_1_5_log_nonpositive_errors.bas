' INV: vbal.6.1.2.10.1.5, func.log
' SOURCE: spec ms-vbal 6.1.2.10.1.5 Log: Number must be greater than zero
' EXPECT: zero=err
' EXPECT: negative=err
Public Sub Main()
    On Error Resume Next
    Dim r As Double
    r = Log(0)
    Debug.Print "zero=" & IIf(Err.Number <> 0, "err", "noerr")
    Err.Clear
    r = Log(-1)
    Debug.Print "negative=" & IIf(Err.Number <> 0, "err", "noerr")
End Sub
