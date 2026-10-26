' INV: stmt.event, stmt.raiseevent, decl.withevents, decl.byref-byval
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/event-statement
' EXPECT: True
'#MODULE Source class
Public Event Ask(ByRef cancel As Boolean)
Public Function TryIt() As Boolean
    Dim c As Boolean
    RaiseEvent Ask(c)
    TryIt = c
End Function
'#MODULE Listener class
Private WithEvents src As Source
Private Sub src_Ask(ByRef cancel As Boolean)
    cancel = True
End Sub
Public Function Run() As Boolean
    Set src = New Source
    Run = src.TryIt()
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim l As New Listener
    Debug.Print l.Run()
End Sub
