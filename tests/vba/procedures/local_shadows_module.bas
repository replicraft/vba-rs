' INV: decl.scope
' SOURCE: derived a local hides a module variable of the same name
' EXPECT:  5  1 
Dim x As Long
Public Sub Main()
    x = 5
    Dim x2 As Long
    x2 = 1
    Debug.Print x; x2
End Sub
