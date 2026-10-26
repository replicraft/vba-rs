' INV: vbal.5.4.2.19, stmt.exit
' SOURCE: spec ms-vbal 5.4.2.19 Exit Property must be inside a property declaration
' ERROR: load
Sub S()
    Exit Property
End Sub
Public Sub Main()
End Sub
