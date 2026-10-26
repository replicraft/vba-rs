' INV: vbal.6.1.2.10.2.1, stmt.randomize
' SOURCE: spec ms-vbal 6.1.2.10.2.1 Randomize: a non-Empty Number MUST be Let-coercible to Double
' EXPECT: err
Public Sub Main()
    On Error Resume Next
    Randomize "not a number"
    If Err.Number <> 0 Then Debug.Print "err" Else Debug.Print "noerr"
End Sub
