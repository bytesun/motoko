open Syntax
open Source
open Effect
open Type   
open Syntaxops   

let breakE l exp typ =
  { it = BreakE (l, exp);
    at = no_region;
    note = {note_eff = eff exp;
            note_typ = typ}
  }

let retE exp typ =
  { it = RetE exp;
    at = no_region;
    note = {note_eff = eff exp;
            note_typ = typ}
  }
  

let assignE exp1 exp2 =
  { it = AssignE (exp1,exp2);
    at = no_region;
    note = {note_eff = Effect.max_eff (eff exp1) (eff exp2);
            note_typ = Type.unit}
  }

let labelE l typT exp =  
  { exp with it = LabelE(l,typT,exp) }

let loopE exp1 exp2Opt =
  { it = LoopE(exp1,exp2Opt);
    at = no_region;
    note = {note_eff = Effect.max_eff (eff exp1)
                         (match exp2Opt with
                          | Some exp2 -> eff exp2
                          | None -> Type.Triv);
            note_typ = Type.unit}
  }
   

(* One traversal for each syntactic category, named by that category *)

let id rho (i:id) =  i

let id_bind rho i =
  match rho with
  | Some (f,x,l,tailCalled) when i.it = f.it -> (* remove shadowed tailbindings *)
    None
  | _ -> rho

let nonTail rho = None

let rec tailexp rho e =
    {e with it = exp' rho e}

and exp rho e  =
    {e with it = exp' None e}

and exp' rho e  = match e.it with
  | VarE i              -> VarE i
  | LitE l              -> e.it
  | PrimE _             -> e.it
  | UnE (uo, e)         -> UnE (uo, exp rho e)
  | BinE (e1, bo, e2)   -> BinE (exp rho e1, bo, exp rho e2)
  | RelE (e1, ro, e2)   -> RelE (exp rho e1, ro, exp rho e2)
  | TupE es             -> TupE (List.map (exp rho) es)
  | ProjE (e, i)        -> ProjE (exp rho e, i)
  | ObjE (s, i, efs)    -> ObjE (s, i, exp_fields rho efs)
  | DotE (e, i)         -> DotE (exp rho e, i)
  | AssignE (e1, e2)    -> AssignE (exp rho e1, exp rho e2)
  | ArrayE es           -> ArrayE (exps rho es)
  | IdxE (e1, e2)       -> IdxE (exp rho e1, exp rho e2)
  | CallE (e1, ts, e2)  -> 
    begin
      match e1.it, ts, rho with
      | VarE f1, [], Some (f2,x,l,tailCalled) when f1.it = f2.it ->
        tailCalled := true;
        (blockE [expD (assignE x (exp rho e2));
                expD (breakE l (tupE []) (typ e))]).it
      | _,_ ,_-> CallE(exp rho e1, ts, exp rho e2)
    end
  | BlockE ds           -> BlockE (decs rho ds)
  | NotE e              -> NotE (exp rho e)
  | AndE (e1, e2)       -> AndE (exp rho e1, tailexp rho e2)
  | OrE (e1, e2)        -> OrE (exp rho e1, tailexp rho e2)
  | IfE (e1, e2, e3)    -> IfE (exp rho e1, tailexp rho e2, tailexp rho e3)
  | SwitchE (e, cs)     -> SwitchE (exp rho e, cases rho cs)
  | WhileE (e1, e2)     -> WhileE (exp rho e1, exp rho e2)
  | LoopE (e1, None)    -> LoopE (exp rho e1, None)
  | LoopE (e1, Some e2) -> LoopE (exp rho e1, Some (exp rho e2))
  | ForE (p, e1, e2)    -> let rho' = pat rho p in
                           ForE (p, exp rho e1, exp rho' e2)
  | LabelE (i, t, e)    -> let rho' = id_bind rho i in
                           LabelE(i, t, exp rho' e)
  | BreakE (i, e)       -> BreakE(id rho i,exp rho e)
  | RetE e              -> RetE (tailexp rho e)
  | AsyncE e            -> AsyncE (exp rho e)
  | AwaitE e            -> AwaitE (exp rho e)
  | AssertE e           -> AssertE (exp rho e)
  | IsE (e, t)          -> IsE (exp rho e, t)
  | AnnotE (e, t)       -> AnnotE (exp rho e, t)
  | DecE d              -> let mk_d, rho' = dec rho d in
                           DecE ({mk_d with it = mk_d.it rho'})
  | OptE e              -> OptE (exp rho e)
  | DeclareE (i, t, e)  -> let rho' = id_bind rho i in
                           DeclareE (i, t, tailexp rho' e)
  | DefineE (i, m, e)   -> DefineE (id rho i, m, exp rho e)
  | NewObjE (s,is)      -> NewObjE (s, List.map (fun (l,i) -> (l,id rho i)) is)
                                   
and exps rho es  = List.map (exp rho) es

and pat rho p =
    let rho = pat' rho p.it in
    rho
     
and pat' rho p = match p with
  | WildP        ->  rho
  | VarP i        ->
     let rho' = id_bind rho i in
     rho'
  | TupP ps       -> pats rho ps 
  | AnnotP (p, t) -> pat rho p 
  | LitP l        -> rho
  | SignP (uo, l) -> rho
  | OptP p        -> pat rho p 
  | AltP (p1, p2) -> assert(Freevars.S.is_empty (snd (Freevars.pat p1)));
                     assert(Freevars.S.is_empty (snd (Freevars.pat p2)));
                     rho

and pats rho ps  =
  match ps with
  | [] -> rho
  | p::ps ->
    let rho' = pat rho p in
    pats rho' ps 

and case rho (c : case) =
  {c with it = case' rho c.it}
and case' rho {pat=p;exp=e} =
  let rho' = pat rho p in
  let e' = tailexp rho' e in
  {pat=p; exp=e'}


and cases rho cs = List.map (case rho) cs

and exp_field rho (ef : exp_field) =
  let (mk_ef,rho) = exp_field' rho ef.it in
    ({ef with it = mk_ef}, rho)

and exp_field' rho {name = n; id = i; exp = e; mut; priv} =
  let rho = id_bind rho i in
  ((fun rho'-> {name = n; id = i; exp = exp rho' e; mut; priv}),
   rho)            

and exp_fields rho efs  = 
  let rec exp_fields_aux rho efs =
    match efs with
    | [] -> ([],rho)
    | ef::efs ->
       let (mk_ef,rho) = exp_field rho ef in
       let (mk_efs,rho) = exp_fields_aux rho efs in
       (mk_ef::mk_efs,rho) in
  let mk_efs,rho = exp_fields_aux rho efs in                           
  List.map (fun mk_ef -> {mk_ef with it = mk_ef.it rho}) mk_efs

and dec rho d =
  let (mk_d,rho') = dec' rho d in
  ({d with it = mk_d}, rho')
                 
and dec' rho d =
  match d.it with
  | ExpD e ->
     (fun rho -> ExpD (tailexp rho e)),
     rho
  | LetD (p, e) ->
     let rho = pat rho p in
     (fun rho' -> LetD(p,exp rho' e)),
     rho              
  | VarD (i, e) ->
     let rho = id_bind rho i in
     (fun rho' -> VarD(i,exp rho' e)),
     rho
  | FuncD (({it=Local;_} as s), id, ([] as typbinds), pat, typT, exp0) ->
    let rho = id_bind rho id in
    (fun rho' ->
      let temp = fresh_id (Mut (typ pat)) in
      let l = fresh_lab () in
      let tailCalled = ref false in
      let rho'' = Some(id,temp,l,tailCalled) in
      let exp0' = tailexp rho'' exp0 in
      if !tailCalled then
        let ids = match typ d with
          | Func(_,_,_,dom,_) -> List.map fresh_id dom         
          | _ -> assert false
        in
        let args = seqP (List.map varP ids) in
        let body =
        blockE [ varD (id_of_exp temp) (seqE ids);
                 expD (loopE
                         (labelE l typT
                            (blockE [letP pat temp;
                                     expD (retE exp0' unit)])) None)
          ] in
        FuncD (s, id, typbinds, args, typT, body)
      else
        FuncD (s, id, typbinds, pat, typT, exp0'))
    ,
    rho
  | FuncD (s, i, tp, p, t, e) ->
     let rho = id_bind rho i in
     (fun rho' ->
       let rho'' = pat rho' p in
       let e' = tailexp rho'' e in
       FuncD(s, i, tp, p, t, e')),
     rho
  | TypD (i, tp, t) -> (* we don't rename type names *)
     (fun rho -> d.it),
     rho
  | ClassD (i, l, tp, s, p, i2, efs) ->
    let rho = id_bind rho i in
     (fun rho' ->
       let rho'' = pat rho' p in
       let rho''' = id_bind rho'' i2 in
       let efs' = exp_fields rho''' efs in
       ClassD(i, l, tp, s, p, i2, efs')),
      rho
       
and decs rho ds =
  let rec tailPosns ds =
    match ds with
    | [] -> (true,[])
    | {it=TypD _;_}::ds ->
      let (b,bs) = tailPosns ds in
      (b,b::bs)
    | d::ds ->
      let (b,bs) = tailPosns ds in
      (false,b::bs)
  in
  let _,tailPosns = tailPosns ds in
  let rec decs_aux rho ds =
    match ds with
    | [] -> ([],rho)
    | d::ds ->
      let (mk_d,rho') = dec rho d in
       let (mk_ds,rho'') = decs_aux rho' ds in
       (mk_d::mk_ds,rho'')
  in
  let mk_ds,rho' = decs_aux rho ds in                           
  List.map2 (fun mk_d inTailPos ->
      let rho'' = if inTailPos then rho' else None in
      {mk_d with it = mk_d.it rho''}) mk_ds tailPosns

 
and prog p:prog = {p with it = decs None p.it}




