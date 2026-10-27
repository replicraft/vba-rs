' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 a parameter receives one argument: naming a parameter that a positional argument already filled is not accepted
' METHOD: by construction
' ERROR: load

Private Function F(a As Long, Optional b As Long = 10, Optional c As Long = 20) As String
    F = a & "," & b & "," & c
End Function
Public Sub Main()
    Debug.Print F(1, a:=2)
End Sub
