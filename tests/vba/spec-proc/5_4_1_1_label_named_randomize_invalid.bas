' INV: vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1.1 an identifier label may not be named Randomize
' XFAIL: wrong not implemented: Randomize is not a reserved word of the lexer, so `Randomize:` is accepted as a label (VBA may read it as the Randomize statement; unconfirmed)
' ERROR: load
Public Sub Main()
Randomize:
    Debug.Print "x"
End Sub
