' INV: stmt.event, stmt.raiseevent, decl.withevents
' SOURCE: spec ms-vbal 5.4.2.20 RaiseEvent statement, 5.3.1.8 event handler declarations
' EXPECT: got 7
'#MODULE Source class
Public Event Fired(ByVal n As Long)
Public Sub Fire(ByVal n As Long)
    RaiseEvent Fired(n)
End Sub
'#MODULE Listener class
Private WithEvents src As Source
Private Sub src_Fired(ByVal n As Long)
    Debug.Print "got " & n
End Sub
Public Sub Run()
    Set src = New Source
    src.Fire 7
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim l As New Listener
    l.Run
End Sub
