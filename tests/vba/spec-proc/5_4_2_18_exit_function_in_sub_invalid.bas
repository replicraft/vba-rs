' INV: vbal.5.4.2.18, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.18 Exit Function must be inside a function declaration
' ERROR: load
Sub S()
    Exit Function
End Sub
Public Sub Main()
End Sub
