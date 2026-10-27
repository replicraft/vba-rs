' INV: vbal.4.1
' SOURCE: derived 4.1 a Private member belongs to its module only; naming it from another module of the project does not compile
' METHOD: by construction
' ERROR: load
'#MODULE Lib standard
Private Function Hidden() As Long
    Hidden = 1
End Function
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print Hidden()
End Sub
