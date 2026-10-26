' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.2 implicit declared type for locals and parameters
' EXPECT: String
' EXPECT: String
' EXPECT: Long
DefStr S
Public Sub Main()
    Dim sv
    Debug.Print TypeName(sv)
    Show "x"
    Dim nn As Long
    Debug.Print TypeName(nn)
End Sub
Private Sub Show(sp)
    Debug.Print TypeName(sp)
End Sub
