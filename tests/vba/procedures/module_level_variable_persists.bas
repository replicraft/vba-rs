' INV: decl.scope
' SOURCE: derived module variables outlive calls
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
