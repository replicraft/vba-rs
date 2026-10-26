' INV: vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1.1 an identifier label may not be named Randomize
' XFAIL: wrong pending
' ERROR: load
Public Sub Main()
Randomize:
    Debug.Print "x"
End Sub
