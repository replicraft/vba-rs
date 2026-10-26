' INV: vbal.5.2.2
' SOURCE: spec ms-vbal 5.2.3.1.5 Def directive scope is the containing module
' EXPECT: here=Empty
' EXPECT: there=String
Public Sub Main()
    Dim ax
    Debug.Print "here=" & TypeName(ax)
    Debug.Print "there=" & Other.T()
End Sub
'#MODULE Other standard
DefStr A
Public Function T() As String
    Dim ax
    T = TypeName(ax)
End Function
