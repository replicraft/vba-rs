' INV: vbal.5.2.4.2
' SOURCE: spec ms-vbal 5.2.4.2 several Implements directives with non-prefix names are valid
' EXPECT: draw
' EXPECT: size
Public Sub Main()
    Dim a As IDraw
    Dim b As ISize
    Dim i As New Impl
    Set a = i
    Set b = i
    a.Go
    b.Go
End Sub
'#MODULE IDraw class
Public Sub Go()
End Sub
'#MODULE ISize class
Public Sub Go()
End Sub
'#MODULE Impl class
Implements IDraw
Implements ISize
Private Sub IDraw_Go()
    Debug.Print "draw"
End Sub
Private Sub ISize_Go()
    Debug.Print "size"
End Sub
