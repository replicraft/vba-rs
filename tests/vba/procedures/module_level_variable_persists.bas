' INV: decl.scope
' SOURCE: spec ms-vbal 2.3 module extent of module variables
' EXPECT:  2 
Dim counter As Long
Sub Bump()
    counter = counter + 1
End Sub
Public Sub Main()
    Bump
    Bump
    Debug.Print counter
End Sub
