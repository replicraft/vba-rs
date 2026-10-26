' INV: vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1.1 label value of an identifier label is its name value (case-insensitive)
' ERROR: load
Public Sub Main()
Here:
    Debug.Print "a"
HERE:
    Debug.Print "b"
End Sub
