' INV: stmt.event, decl.withevents
' SOURCE: spec ms-vbal 5.4.3.9 Set statement detaches WithEvents handlers, 5.4.2.20 RaiseEvent statement
' EXPECT: got 1
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
    src.Fire 1
    Dim keep As Source
    Set keep = src
    Set src = Nothing
    keep.Fire 2
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim l As New Listener
    l.Run
End Sub
