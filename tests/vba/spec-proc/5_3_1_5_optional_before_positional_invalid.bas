' INV: vbal.5.3.1.5
' SOURCE: spec ms-vbal 5.3.1.5 parameter-list grammar: positional parameters precede optional ones
' ERROR: load
Sub S(Optional a As Long, b As Long)
End Sub
Public Sub Main()
End Sub
