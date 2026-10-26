' INV: vbal.5.4, vbal.5.4.1.1
' SOURCE: spec ms-vbal 5.4.1.1 label values within a procedure are unique
' ERROR: load
Public Sub Main()
Here:
    Debug.Print "a"
Here:
    Debug.Print "b"
End Sub
