' INV: vbal.6.1.2.10.1.4, func.exp
' SOURCE: spec ms-vbal 6.1.2.10.1.4 Exp: Number above 709.782712893 is an error; the limit itself is allowed
' EXPECT: ok=True
' EXPECT: err
Public Sub Main()
    Debug.Print "ok=" & (Exp(709.78) > 1E+307)
    On Error Resume Next
    Dim r As Double
    r = Exp(710)
    If Err.Number <> 0 Then Debug.Print "err" Else Debug.Print "noerr"
End Sub
