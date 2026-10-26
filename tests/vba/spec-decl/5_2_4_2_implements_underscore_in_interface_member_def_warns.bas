' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 interface class public members cannot contain an underscore (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE IBad class
Public Sub Do_It()
End Sub
'#MODULE Impl class
Implements IBad
Private Sub IBad_Do_It()
End Sub
