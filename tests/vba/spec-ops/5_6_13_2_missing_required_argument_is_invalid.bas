' INV: vbal.5.6.13.2
' SOURCE: derived 5.6.13.2 every non-optional parameter must receive an argument (VBA: "Argument not optional")
' METHOD: by construction
' ERROR: load

Private Function F(a As Long, Optional b As Long = 10, Optional c As Long = 20) As String
    F = a & "," & b & "," & c
End Function
Public Sub Main()
    Debug.Print F(b:=1)
End Sub
