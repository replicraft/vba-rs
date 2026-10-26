' INV: stmt.event, stmt.raiseevent, decl.withevents
' SOURCE: spec ms-vbal 5.4.2.20 RaiseEvent statement, 5.3.1.8 event handler declarations
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
