' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 the positional arguments come before the named ones; a positional argument after a named argument is not accepted
' METHOD: by construction
' ERROR: load

Private Function F(a As Long, Optional b As Long = 10, Optional c As Long = 20) As String
    F = a & "," & b & "," & c
End Function
Public Sub Main()
    Debug.Print F(a:=1, 2)
End Sub
