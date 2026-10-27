' INV: vbal.5.6.4
' SOURCE: derived 5.6.4 after As the name is bound as a type: a Public type of another module may be named qualified by its module
' METHOD: by hand
' EXPECT:  3 
' EXPECT:  4 
'#MODULE Shapes standard
Public Type Pt
    X As Long
    Y As Long
End Type
'#MODULE Module1 standard
Public Sub Main()
    Dim p As Shapes.Pt
    p.X = 3
    p.Y = 4
    Debug.Print p.X
    Debug.Print p.Y
End Sub
