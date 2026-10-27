' INV: vbal.5.2.3.6
' SOURCE: spec ms-vbal 5.2.3.6 circular dependency through Implements directives is illegal (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Public Sub Main()
    Debug.Print "ok"
End Sub
'#MODULE ClsA class
Implements ClsB
Private Sub ClsB_Hi()
End Sub
'#MODULE ClsB class
Implements ClsA
Private Sub ClsA_Hi()
End Sub
Public Sub Hi()
End Sub
