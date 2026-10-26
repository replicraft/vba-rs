' INV: vbal.5.4.2.1, stmt.call
' SOURCE: spec ms-vbal 5.4.2.1 Call whose argument list is incompatible with the procedure is invalid
' ERROR: load
Sub S(a As Long)
End Sub
Public Sub Main()
    Call S(1, 2)
End Sub
