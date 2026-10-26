' INV: stmt.event, stmt.raiseevent, decl.withevents
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/event-statement
' EXPECT: A
' EXPECT: Bx
'#MODULE Source class
Public Event A()
Public Event B(ByVal s As String)
Public Sub Go()
    RaiseEvent A
    RaiseEvent B("x")
End Sub
'#MODULE Listener class
Private WithEvents src As Source
Private Sub src_A()
    Debug.Print "A"
End Sub
Private Sub src_B(ByVal s As String)
    Debug.Print "B" & s
End Sub
Public Sub Run()
    Set src = New Source
    src.Go
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim l As New Listener
    l.Run
End Sub
