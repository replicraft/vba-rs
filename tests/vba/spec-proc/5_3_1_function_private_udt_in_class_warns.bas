' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 function in class module may not return a private UDT (spec-strict: a warning by default, the program runs)
' EXPECT: ok
'#MODULE Cls class
Private Type Pt
    X As Long
End Type
Public Function F() As Pt
End Function
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "ok"
End Sub
