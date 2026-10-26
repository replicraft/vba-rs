' INV: vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1.1 an identifier label may not be named Randomize
' XFAIL: wrong not implemented: statement keywords as label names
' ERROR: load
Public Sub Main()
Randomize:
    Debug.Print "x"
End Sub
