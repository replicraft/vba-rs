' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 no implemented interface name prefix may be the initial text of another (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE A class
Public Sub Hi()
End Sub
'#MODULE A_B class
Public Sub Yo()
End Sub
'#MODULE Impl class
Implements A
Implements A_B
Private Sub A_Hi()
End Sub
Private Sub A_B_Yo()
End Sub
