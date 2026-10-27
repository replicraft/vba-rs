' INV: vbal.4.1
' SOURCE: derived 4.1 a standard module exists once per project: its variables are shared by every caller, while each instance of a class module has its own
' METHOD: by hand: three increments of the shared counter; two instances count separately
' EXPECT:  1 
' EXPECT:  2 
' EXPECT:  3 
' EXPECT:  2 
' EXPECT:  1 
'#MODULE Counters standard
Public N As Long
Public Sub Inc()
    N = N + 1
End Sub
'#MODULE Cell class
Public V As Long
Public Sub Inc()
    V = V + 1
End Sub
'#MODULE Module1 standard
Private Sub Twice()
    Counters.Inc
    Debug.Print Counters.N
End Sub
Public Sub Main()
    Twice
    Twice
    Twice
    Dim a As New Cell, b As New Cell
    a.Inc
    a.Inc
    b.Inc
    Debug.Print a.V
    Debug.Print b.V
End Sub
