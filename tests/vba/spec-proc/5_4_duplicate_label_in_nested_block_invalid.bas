' INV: vbal.5.4
' SOURCE: spec ms-vbal 5.4 labels in contained statement blocks must be unique too
' ERROR: load
Public Sub Main()
Here:
    If True Then
Here:
        Debug.Print "b"
    End If
End Sub
