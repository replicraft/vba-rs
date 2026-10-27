' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 positional arguments beyond the last parameter are not accepted (VBA: "Wrong number of arguments")
' METHOD: by construction
' ERROR: load

Private Function F(a As Long, Optional b As Long = 10, Optional c As Long = 20) As String
    F = a & "," & b & "," & c
End Function
Public Sub Main()
    Debug.Print F(1, 2, 3, 4)
End Sub
